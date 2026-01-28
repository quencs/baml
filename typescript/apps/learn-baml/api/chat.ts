import type { VercelRequest, VercelResponse } from '@vercel/node';
import OpenAI from 'openai';
import { b } from '../baml_client/baml_client';
import type { DocContext, Message } from '../baml_client/baml_client';

interface ChatRequest {
  message: string;
  prev_messages?: Array<{ role: 'user' | 'assistant'; text: string }>;
  stream?: boolean;
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
  if (!response.ok) {
    throw new Error(`Failed to load embeddings: ${response.status}`);
  }
  cachedIndex = await response.json();

  return cachedIndex!;
}

function cosineSimilarity(a: number[], b: number[]): number {
  if (a.length !== b.length) {
    throw new Error(`Vector length mismatch: ${a.length} vs ${b.length}`);
  }

  let dotProduct = 0;
  let normA = 0;
  let normB = 0;

  for (let i = 0; i < a.length; i++) {
    dotProduct += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }

  const denominator = Math.sqrt(normA) * Math.sqrt(normB);
  if (denominator === 0) return 0;

  return dotProduct / denominator;
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
  if (!response.data || response.data.length === 0) {
    throw new Error('No embedding returned from OpenAI');
  }
  return response.data[0].embedding;
}

export default async function handler(req: VercelRequest, res: VercelResponse) {
  if (req.method !== 'POST') {
    return res.status(405).send('Method not allowed');
  }

  // TODO: Add rate limiting here
  // Consider using Vercel's rate limiting or a service like Upstash
  // Example: Check IP against rate limit store, return 429 if exceeded

  try {
    const { message, prev_messages = [], stream = false }: ChatRequest = req.body;

    if (!message?.trim()) {
      return res.status(400).json({ error: 'Message is required' });
    }

    if (message.length > 2000) {
      return res.status(400).json({ error: 'Message too long (max 2000 characters)' });
    }

    if (prev_messages.length > 20) {
      return res.status(400).json({ error: 'Too many previous messages (max 20)' });
    }

    if (!process.env.OPENAI_API_KEY) {
      return res.status(503).json({ error: 'Service temporarily unavailable' });
    }
    const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });

    // Get the base URL from the request
    const protocol = req.headers['x-forwarded-proto'] || 'http';
    const host = req.headers.host || 'localhost:3000';
    const baseUrl = `${protocol}://${host}`;

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

    const bamlInput = {
      query: message,
      context_docs: contextDocs,
      prev_messages: prevMessages,
    };

    // 5. Call BAML function (streaming or non-streaming)
    if (stream) {
      // Set up SSE headers
      res.setHeader('Content-Type', 'text/event-stream');
      res.setHeader('Cache-Control', 'no-cache');
      res.setHeader('Connection', 'keep-alive');
      res.setHeader('X-Accel-Buffering', 'no');

      // Send initial doc scores event
      res.write(`data: ${JSON.stringify({ type: 'doc_scores', scores: relevantDocs.map(d => d.score) })}\n\n`);

      const bamlStream = b.stream.AskBaml(bamlInput);

      // Stream partial results
      for await (const partial of bamlStream) {
        res.write(`data: ${JSON.stringify({
          type: 'partial',
          answer: partial.answer ?? '',
          citations: (partial.citations ?? []).map(c => ({
            title: c?.title ?? '',
            url: c?.url ?? '',
            relevance: c?.relevance ?? 'Low',
          })),
          suggested_questions: partial.suggested_questions ?? [],
        })}\n\n`);
      }

      // Get final response and send completion event
      const final = await bamlStream.getFinalResponse();
      res.write(`data: ${JSON.stringify({
        type: 'done',
        answer: final.answer,
        citations: final.citations.map(c => ({
          title: c.title,
          url: c.url,
          relevance: c.relevance,
        })),
        suggested_questions: final.suggested_questions ?? [],
      })}\n\n`);

      res.end();
    } else {
      // Non-streaming response (original behavior)
      const result = await b.AskBaml(bamlInput);

      return res.status(200).json({
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
    }
  } catch (error) {
    console.error('Chat error:', error);
    return res.status(500).json({ error: 'Failed to process request' });
  }
}
