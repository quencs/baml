import OpenAI from 'openai';
import { b } from '../baml_client/baml_client';
import type { DocContext, Message } from '../baml_client/baml_client';

interface ChatRequest {
  message: string;
  prev_messages?: Array<{ role: 'user' | 'assistant'; text: string }>;
}

interface EmbeddingsIndex {
  model: string;
  dimensions: number;
  chunks: Array<{
    id: string;
    title: string;
    url: string;
    content: string;
    section?: string;
    embedding: number[];
  }>;
}

interface SearchResult {
  id: string;
  title: string;
  url: string;
  content: string;
  section?: string;
  score: number;
}

let cachedIndex: EmbeddingsIndex | null = null;

async function loadEmbeddingsIndex(baseUrl: string): Promise<EmbeddingsIndex> {
  if (cachedIndex) return cachedIndex;

  const response = await fetch(`${baseUrl}/embeddings.json`);
  cachedIndex = await response.json();

  return cachedIndex!;
}

function cosineSimilarity(a: number[], b: number[]): number {
  let dotProduct = 0;
  let normA = 0;
  let normB = 0;

  for (let i = 0; i < a.length; i++) {
    dotProduct += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }

  return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
}

function searchByEmbedding(
  queryEmbedding: number[],
  index: EmbeddingsIndex,
  topK = 5
): SearchResult[] {
  const scored = index.chunks.map(chunk => ({
    ...chunk,
    score: cosineSimilarity(queryEmbedding, chunk.embedding),
  }));

  scored.sort((a, b) => b.score - a.score);

  return scored.slice(0, topK).map(({ embedding, ...rest }) => rest);
}

async function embedQuery(
  query: string,
  openai: OpenAI,
  model = 'text-embedding-3-small'
): Promise<number[]> {
  const response = await openai.embeddings.create({
    model,
    input: query,
  });
  return response.data[0].embedding;
}

export default async function handler(req: Request) {
  if (req.method !== 'POST') {
    return new Response('Method not allowed', { status: 405 });
  }

  try {
    const { message, prev_messages = [] }: ChatRequest = await req.json();

    if (!message?.trim()) {
      return Response.json({ error: 'Message is required' }, { status: 400 });
    }

    const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });

    // Get the base URL from the request
    const url = new URL(req.url);
    const baseUrl = `${url.protocol}//${url.host}`;

    // 1. Load embeddings index
    const index = await loadEmbeddingsIndex(baseUrl);

    // 2. Embed the query
    const queryEmbedding = await embedQuery(message, openai, index.model);

    // 3. Search for relevant docs
    const relevantDocs = searchByEmbedding(queryEmbedding, index, 5);

    // 4. Build BAML input
    const contextDocs: DocContext[] = relevantDocs.map(doc => ({
      title: doc.title,
      url: doc.url,
      content: doc.content,
      section: doc.section ?? null,
    }));

    const prevMessages: Message[] = prev_messages.map(m => ({
      role: m.role,
      text: m.text,
    }));

    // 5. Call BAML function
    const result = await b.AskBaml({
      query: message,
      context_docs: contextDocs,
      prev_messages: prevMessages,
    });

    return Response.json({
      answer: result.answer,
      citations: result.citations.map(c => ({
        title: c.title,
        url: c.url,
        relevance: c.relevance,
      })),
      suggested_questions: result.suggested_questions ?? [],
      _debug: {
        doc_scores: relevantDocs.map(d => d.score),
      },
    });
  } catch (error) {
    console.error('Chat error:', error);
    return Response.json(
      { error: 'Failed to process request' },
      { status: 500 }
    );
  }
}
