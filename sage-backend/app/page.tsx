'use client';
import { useEffect, useState } from 'react';
import { type QueryResponse, submitQuery } from './actions/query';
import type { QueryRequest } from './types';
async function hashQueryRequest(request: QueryRequest): Promise<string> {
  const sortedRequest = JSON.stringify(request, Object.keys(request).sort());
  const encoder = new TextEncoder();
  const data = encoder.encode(sortedRequest);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
}

// Browser localStorage cache utilities
const CACHE_PREFIX = 'query_cache_';

function getCachedResult(cacheKey: string): QueryResponse | null {
  try {
    const cached = localStorage.getItem(CACHE_PREFIX + cacheKey);
    return cached ? JSON.parse(cached) : null;
  } catch {
    return null;
  }
}

function setCachedResult(cacheKey: string, result: QueryResponse): void {
  try {
    localStorage.setItem(CACHE_PREFIX + cacheKey, JSON.stringify(result));
  } catch {
    // localStorage might be full or unavailable, ignore silently
  }
}

function invalidateCache(cacheKey: string): void {
  try {
    localStorage.removeItem(CACHE_PREFIX + cacheKey);
  } catch {
    // localStorage might be unavailable, ignore silently
  }
}

async function cachedSubmitQuery(
  queryRequest: QueryRequest,
): Promise<QueryResponse> {
  const cacheKey = await hashQueryRequest(queryRequest);

  // Check if result is already cached in localStorage
  const cachedResult = getCachedResult(cacheKey);
  if (cachedResult) {
    return cachedResult;
  }

  // If not cached, call the original function and cache the result
  const result = await submitQuery(queryRequest);
  setCachedResult(cacheKey, result);

  return result;
}

// Define the placeholder queries with optional inspect notes
const PLACEHOLDER_QUERIES: { input: QueryRequest; inspectNotes?: string }[] = [
  {
    input: {
      query:
        'can I load enums or classes from a saved state, (after defining dynamically previously, then saving)',
      prev_messages: [],
    },
  },
  {
    input: {
      query:
        'cal I load enums from a class not created in baml? (for instance saved state of dynamic types)',
      prev_messages: [],
    },
  },
  {
    input: {
      query: 'Can I bring my own LLM client?',
      prev_messages: [],
    },
  },
  {
    input: {
      query: 'How do I see the prompt that rendered in the response',
      prev_messages: [],
    },
  },
  {
    input: {
      query: 'building a test to incorporate a test image file',
      prev_messages: [],
    },
  },
  {
    input: {
      query: 'Can I use Excel sheets as an input',
      prev_messages: [],
    },
  },
  {
    input: {
      query: 'hi how do i do this',
      prev_messages: [],
    },
  },
  {
    input: {
      query: 'How can I type-hint a list of 3-lenght tuples of strings?',
      prev_messages: [],
    },
  },
  {
    input: {
      query: 'i dont understand why this is required. really. Give an example',
      prev_messages: [],
    },
  },
  {
    input: {
      query:
        "I'm not a developer or software engineer but can i still learn baml?",
      prev_messages: [],
    },
  },
  {
    input: {
      query: 'what do i have to know to learn baml?',
      prev_messages: [],
    },
  },
  {
    input: {
      query: 'How can I control retries and fallback?',
      prev_messages: [],
    },
  },
  {
    input: {
      query:
        'Help me understand this code:\n\n\ndef _pick_best_categories(text: str, categories: list[Category]) -> list[Category]:\n    tb = TypeBuilder()\n    for k in categories:\n        val = tb.Category.add_value(k.name)\n        val.description(k.llm_description)\n    selected_categories = b.PickBestCategories(text, count=3, baml_options={ "tb": tb })\n    return [category for category in categories if category.name in selected_categories]',
      prev_messages: [],
    },
  },
  {
    input: {
      query:
        'i am using provider "openai-generic" to tlak to ollama what options am i allowed ot pass?',
      prev_messages: [],
    },
  },
  {
    input: {
      query: 'can you give an example of how the type alias is used?',
      prev_messages: [],
    },
    inspectNotes:
      'answer should talk about the `@alias` feature of BAML, but often treats' +
      'this as a "type alias" in the generic sense of the phrase',
  },
  {
    input: {
      query: 'can you make an alias dynamically for an existing enum using tb?',
      prev_messages: [],
    },
    inspectNotes: 'answer should reference TypeBuilder, not baml code',
  },
  {
    input: {
      query: 'Is there a retry after a BamlValidationError?',
      prev_messages: [],
    },
  },
];

interface QueryResult {
  queryData: { input: QueryRequest; inspectNotes?: string };
  response: QueryResponse | null;
  error: string | null;
  isLoading: boolean;
}

export default function Home() {
  const [queryResults, setQueryResults] = useState<QueryResult[]>(
    PLACEHOLDER_QUERIES.map((queryData) => ({
      queryData,
      response: null,
      error: null,
      isLoading: false,
    })),
  );

  // Function to run a single query
  const runQuery = async (index: number) => {
    setQueryResults((prev) =>
      prev.map((item, i) =>
        i === index
          ? { ...item, isLoading: true, error: null, response: null }
          : item,
      ),
    );

    try {
      // Invalidate cache before making new request
      const cacheKey = await hashQueryRequest(PLACEHOLDER_QUERIES[index].input);
      invalidateCache(cacheKey);
      
      const response = await cachedSubmitQuery(
        PLACEHOLDER_QUERIES[index].input,
      );

      setQueryResults((prev) =>
        prev.map((item, i) =>
          i === index ? { ...item, response, isLoading: false } : item,
        ),
      );
    } catch (err) {
      setQueryResults((prev) =>
        prev.map((item, i) =>
          i === index
            ? {
                ...item,
                error: err instanceof Error ? err.message : 'An error occurred',
                isLoading: false,
              }
            : item,
        ),
      );
    }
  };

  // Function to retry all queries
  const retryAllQueries = async () => {
    await Promise.all(PLACEHOLDER_QUERIES.map((_, index) => runQuery(index)));
  };

  // Run all queries on mount
  useEffect(() => {
    Promise.all(PLACEHOLDER_QUERIES.map((_, index) => runQuery(index)));
  }, []);

  return (
    <div className="font-sans min-h-screen p-8 pb-20 sm:p-20">
      <main className="w-full max-w-7xl mx-auto">
        <h1 className="text-3xl font-bold mb-8 text-gray-900 dark:text-gray-100">
          BAML Query Test Results
        </h1>

        <div className="overflow-x-auto">
          <table className="w-full border-collapse bg-white dark:bg-gray-800 rounded-lg shadow-lg">
            <thead>
              <tr className="border-b border-gray-200 dark:border-gray-700">
                <th className="px-4 py-4 text-left text-sm font-semibold text-gray-900 dark:text-gray-100">
                  <div className="flex flex-col gap-1">
                    <button
                      onClick={retryAllQueries}
                      className="text-xs px-2 py-1 bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors"
                    >
                      Retry All
                    </button>
                  </div>
                </th>
                <th className="px-6 py-4 text-left text-sm font-semibold text-gray-900 dark:text-gray-100 w-1/4">
                  Query
                </th>
                <th className="px-6 py-4 text-left text-sm font-semibold text-gray-900 dark:text-gray-100 w-2/3">
                  Result
                </th>
                <th className="px-6 py-4 text-left text-sm font-semibold text-gray-900 dark:text-gray-100 w-96">
                  Inspect Notes
                </th>
              </tr>
            </thead>
            <tbody>
              {queryResults.map((item, index) => (
                <tr
                  key={index}
                  className={`border-b border-gray-200 dark:border-gray-700 hover:bg-blue-50 dark:hover:bg-blue-900/20 ${
                    index % 2 === 0
                      ? 'bg-white dark:bg-gray-800'
                      : 'bg-gray-100 dark:bg-gray-700'
                  }`}
                >
                  <td className="px-4 py-4 align-middle">
                    <button
                      onClick={() => runQuery(index)}
                      disabled={item.isLoading}
                      className="text-xs px-3 py-1 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white rounded transition-colors"
                    >
                      {item.isLoading ? 'Loading...' : 'Retry'}
                    </button>
                  </td>
                  <td className="px-6 py-4 align-middle">
                    <p className="text-sm text-gray-700 dark:text-gray-300">
                      {item.queryData.input.query}
                    </p>
                  </td>
                  <td className="px-6 py-4">
                    {item.isLoading && (
                      <div className="flex items-center space-x-2">
                        <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
                        <span className="text-sm text-gray-600 dark:text-gray-400">
                          Loading...
                        </span>
                      </div>
                    )}

                    {item.error && (
                      <div className="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded">
                        <p className="text-sm text-red-800 dark:text-red-200">
                          {item.error}
                        </p>
                      </div>
                    )}

                    {item.response && (
                      <div className="flex gap-4 flex-row items-center">
                        {/* Answer */}
                        <div className="flex-1">
                          {item.response.answer && (
                            <div className="p-3 bg-green-50 dark:bg-green-900/20 rounded">
                              <p className="text-sm text-gray-800 dark:text-gray-200">
                                {item.response.answer}
                              </p>
                            </div>
                          )}
                        </div>

                        {/* Relevant Documents */}
                        {item.response.ranked_docs &&
                          item.response.ranked_docs.length > 0 && (
                            <div className="w-96">
                              <p className="text-xs font-semibold text-gray-600 dark:text-gray-400">
                                Related Documents:
                              </p>
                              <div>
                                {item.response.ranked_docs.map((doc) => (
                                  <div
                                    key={doc.url}
                                    className="p-2 bg-gray-50 dark:bg-gray-700/50 rounded text-xs mb-1"
                                  >
                                    <div className="flex items-center justify-between gap-2">
                                      <div className="flex-1 min-w-0">
                                        <p className="font-medium text-gray-700 dark:text-gray-300 truncate text-xs">
                                          {doc.title}
                                        </p>
                                        <a
                                          href={doc.url}
                                          target="_blank"
                                          rel="noopener noreferrer"
                                          className="text-blue-600 dark:text-blue-400 hover:underline truncate block text-xs"
                                        >
                                          {doc.url}
                                        </a>
                                      </div>
                                      <span
                                        className={`flex-shrink-0 px-2 py-0.5 rounded text-xs font-medium ${
                                          doc.relevance === 'very-relevant'
                                            ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300'
                                            : doc.relevance === 'relevant'
                                              ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300'
                                              : 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300'
                                        }`}
                                      >
                                        {doc.relevance}
                                      </span>
                                    </div>
                                  </div>
                                ))}
                              </div>
                            </div>
                          )}
                      </div>
                    )}
                  </td>
                  <td className="px-6 py-4 align-middle">
                    {item.queryData.inspectNotes && (
                      <div className="p-2 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded">
                        <p className="text-xs text-yellow-800 dark:text-yellow-200">
                          {item.queryData.inspectNotes}
                        </p>
                      </div>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </main>
    </div>
  );
}
