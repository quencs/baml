'use server';

import { z } from 'zod';
import { b } from '../../baml_client';
import type { QueryRequest } from '../types';
import { searchPinecone } from './rag';

// Define the Zod schema for the response
const RankedDocSchema = z.object({
  title: z.string(),
  url: z.string(),
  relevance: z.number(),
});

const QueryResponseSchema = z.object({
  ranked_docs: z.array(RankedDocSchema),
  answer: z.string().optional().or(z.null()),
});

export type QueryResponse = z.infer<typeof QueryResponseSchema>;

export async function submitQuery(
  queryRequest: QueryRequest,
): Promise<QueryResponse> {
  const docs = await searchPinecone(queryRequest.query);
  const rankedDocs = docs.map((doc) => ({
    title: (doc.metadata?.title ?? '') as string,
    url: (doc.metadata?.slug ?? '') as string,
    relevance: doc.score ?? 0,
  }));

  const plan = await b.PlanQuery({
    text: queryRequest.query,
    language_preference: queryRequest.language_preference,
    context_docs: rankedDocs.map((doc) => ({
      title: doc.title,
      body: doc.url,
      relevance_score: doc.relevance,
    })),
    prev_messages: queryRequest.prev_messages,
  });

  for (const doc of rankedDocs) {
    console.log({ url: doc.url, relevance: doc.relevance });
  }

  // TODO: implement auto-navigation based on LLM tagging as "very-relevant"

  const resp = {
    answer: plan.answer,
    ranked_docs: rankedDocs.map((doc) => ({
      title: doc.title,
      url: doc.url,
      relevance: doc.relevance,
    })),
  };

  return resp;
}
