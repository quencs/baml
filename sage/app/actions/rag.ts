'use server';
import { Pinecone } from '@pinecone-database/pinecone';
import OpenAI from 'openai';

const openaiClient = new OpenAI({
  apiKey: process.env.OPENAI_API_KEY ?? '',
});

const pineconeClient = new Pinecone({
  apiKey: process.env.PINECONE_API_KEY ?? '',
});

const pineconeIndex = pineconeClient.Index('baml-index');

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

function chunkMarkdown(text: string, maxChunkSize = 16000): string[] {
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
export async function searchPinecone(query: string, count = 5) {
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

// Copy vectors from baml-index to baml-index-sage
export async function copyPineconeIndex() {
  const sourceIndex = pineconeClient.Index('baml-index');
  const targetIndex = pineconeClient.Index('baml-index-sage');

  try {
    // Get index statistics to understand the data size
    const stats = await sourceIndex.describeIndexStats();
    console.log('Source index stats:', stats);

    const totalVectors = stats.totalRecordCount || 0;
    if (totalVectors === 0) {
      console.log('No vectors found in source index');
      return;
    }

    console.log(
      `Copying ${totalVectors} vectors from baml-index to baml-index-sage...`,
    );

    // Clear the target index first
    // try {
    //   await targetIndex.deleteAll();
    //   console.log('Cleared target index');
    // } catch (e) {
    //   console.log('Target index was already empty or error clearing:', e);
    // }

    // We need to query in batches since Pinecone doesn't have a "list all" operation
    // We'll use a dummy query to get all vectors
    const batchSize = 1000;
    let copiedCount = 0;

    // Get all unique namespaces first
    const namespaces = Object.keys(stats.namespaces || { '': stats });

    for (const namespace of namespaces) {
      console.log(`Processing namespace: ${namespace || 'default'}`);

      // Query with a zero vector to get vectors (this is a workaround)
      // Since we can't list all vectors, we'll query with high topK
      const queryOptions = {
        topK: Math.min(batchSize, 10000), // Pinecone max is 10000
        includeMetadata: true,
        includeValues: true,
      };

      // We need to provide a vector for the query, so we'll use a dummy vector
      // with the same dimensions. Let's get the dimension from the first vector
      let vectorDimension = 3072; // Default for text-embedding-3-large

      try {
        // Try to get a sample vector to determine dimensions
        const sampleQueryOptions = {
          ...queryOptions,
          vector: new Array(vectorDimension).fill(0),
          topK: 1,
        };

        const sampleQuery = namespace
          ? await sourceIndex.namespace(namespace).query(sampleQueryOptions)
          : await sourceIndex.query(sampleQueryOptions);

        if (sampleQuery.matches.length > 0) {
          vectorDimension =
            sampleQuery.matches[0].values?.length || vectorDimension;
        }
      } catch (e) {
        console.log(
          'Could not determine vector dimension, using default:',
          vectorDimension,
        );
      }

      // Query all vectors in this namespace
      const finalQueryOptions = {
        ...queryOptions,
        vector: new Array(vectorDimension).fill(0),
        topK: 10000, // Get as many as possible
      };

      const queryResult = namespace
        ? await sourceIndex.namespace(namespace).query(finalQueryOptions)
        : await sourceIndex.query(finalQueryOptions);

      const vectors = queryResult.matches;
      console.log(
        `Found ${vectors.length} vectors in namespace: ${namespace || 'default'}`,
      );

      if (vectors.length === 0) continue;

      // Prepare records for upsert
      const records = vectors.map((match) => ({
        id: match.id,
        values: match.values || [],
        metadata: match.metadata || {},
      }));

      // Upsert in smaller batches
      const upsertBatchSize = 100;
      for (let i = 0; i < records.length; i += upsertBatchSize) {
        const batch = records.slice(i, i + upsertBatchSize);

        if (namespace) {
          await targetIndex.namespace(namespace).upsert(batch);
        } else {
          await targetIndex.upsert(batch);
        }

        copiedCount += batch.length;

        console.log(`Copied ${copiedCount}/${totalVectors} vectors...`);
      }
    }

    console.log(
      `Successfully copied ${copiedCount} vectors from baml-index to baml-index-sage`,
    );

    // Verify the copy
    const targetStats = await targetIndex.describeIndexStats();
    console.log('Target index stats after copy:', targetStats);
  } catch (error) {
    console.error('Error copying Pinecone index:', error);
    throw error;
  }
}
