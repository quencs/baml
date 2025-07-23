'use server';
import OpenAI from 'openai';
import { Pinecone } from '@pinecone-database/pinecone';

const openaiClient = new OpenAI({
  apiKey: process.env.OPENAI_API_KEY ?? '',
});

const pineconeClient = new Pinecone({
  apiKey: process.env.PINECONE_API_KEY ?? '',
});

const pineconeIndex = pineconeClient.Index('baml-index-sage');

interface FernDoc {
  slug: string;
  path: string;
  body: string;
  chunkIndex?: number;
}

interface EmbeddingWithMetadata {
  embedding: number[];
  document: FernDoc;
}

function chunkMarkdown(text: string, maxChunkSize: number = 16000): string[] {
  // Split on H1 and H2 headers
  const headerRegex = /^#{1,2}\s+.+$/gm;
  const sections = text.split(headerRegex);
  const headers = text.match(headerRegex) || [];

  const chunks: string[] = [];
  let currentChunk = '';

  // Combine headers with their content
  for (let i = 0; i < sections.length; i++) {
    const header = headers[i - 1] || ''; // First section might not have header
    const content = sections[i];
    const combined = `${header}\n${content}`.trim();

    if (combined.length <= maxChunkSize) {
      // If current chunk + combined would exceed limit, start new chunk
      if ((currentChunk + combined).length > maxChunkSize && currentChunk) {
        chunks.push(currentChunk.trim());
        currentChunk = combined;
      } else {
        currentChunk = currentChunk
          ? `${currentChunk}\n\n${combined}`
          : combined;
      }
    } else {
      // If single section is too large, split by paragraphs
      if (currentChunk) {
        chunks.push(currentChunk.trim());
        currentChunk = '';
      }

      const paragraphs = combined.split(/\n\s*\n/);
      let paragraphChunk = '';

      for (const paragraph of paragraphs) {
        if ((paragraphChunk + paragraph).length > maxChunkSize) {
          if (paragraphChunk) chunks.push(paragraphChunk.trim());
          paragraphChunk = paragraph;
        } else {
          paragraphChunk = paragraphChunk
            ? `${paragraphChunk}\n\n${paragraph}`
            : paragraph;
        }
      }

      if (paragraphChunk) chunks.push(paragraphChunk.trim());
    }
  }

  if (currentChunk) chunks.push(currentChunk.trim());
  return chunks;
}

// TODO store the whole document even for each chunk?
export async function populatePinecone() {
  const fs = require('fs');
  const path = require('path');
  const docs = JSON.parse(fs.readFileSync('./fern.json', 'utf8'));

  // Filter out changelog documents
  const filteredDocs = docs.filter(
    (doc: FernDoc) => !doc.slug.includes('/changelog'),
  );

  // Delete all existing records once before starting
  try {
    await pineconeIndex.deleteAll();
  } catch (e) {
    console.log('No existing records to delete');
  }

  // Process docs in batches of 10
  for (let i = 0; i < filteredDocs.length; i += 10) {
    const batch = filteredDocs.slice(i, i + 10);

    // First, create all chunks for each document
    const chunkedDocs: { doc: FernDoc; chunk: string; chunkIndex: number }[] =
      batch.flatMap((doc: FernDoc) => {
        const chunks = chunkMarkdown(doc.body);
        return chunks.map((chunk, chunkIndex) => ({
          doc,
          chunk,
          chunkIndex,
        }));
      });

    // Then, generate embeddings for all chunks
    const embeddingsWithMetadata: EmbeddingWithMetadata[] = await Promise.all(
      chunkedDocs.map(async ({ doc, chunk, chunkIndex }) => {
        const embeddingResponse = await openaiClient.embeddings.create({
          model: 'text-embedding-3-large',
          input: chunk,
        });

        return {
          embedding: embeddingResponse.data[0].embedding,
          document: {
            slug: doc.slug,
            path: doc.path,
            body: chunk,
            chunkIndex,
          },
        };
      }),
    );

    // Prepare records for Pinecone using the combined data
    const records = embeddingsWithMetadata.map(({ embedding, document }) => ({
      id: `${document.slug}-chunk-${document.chunkIndex}`,
      values: embedding,
      metadata: {
        slug: document.slug,
        path: document.path,
        body: document.body,
      },
    }));

    // Upsert in batches of 100
    for (let j = 0; j < records.length; j += 100) {
      const upsertBatch = records.slice(j, j + 100);
      await pineconeIndex.upsert(upsertBatch);
    }
  }
}

// RAG using pinecone
export async function searchPinecone(query: string, count: number = 5) {
  const results = await pineconeIndex.query({
    vector: await openaiClient.embeddings
      .create({
        model: 'text-embedding-3-large',
        input: query,
      })
      .then((res) => res.data[0].embedding),
    topK: count,
    includeMetadata: true,
  });
  console.log('Got matches', results.matches.length);
  // console.log(results.matches);
  return results.matches;
}
