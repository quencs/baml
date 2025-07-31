'use client';
import { useEffect, useState } from 'react';
import { type QueryResponse, submitQuery } from './actions/query';

// Define the 5 placeholder queries
const PLACEHOLDER_QUERIES = [
  'How do I define a function in BAML?',
  'What are BAML types and how do I use them?',
  'How to connect to OpenAI using BAML clients?',
  'What is the difference between class and enum in BAML?',
  'How do I test BAML functions?',
];

interface QueryResult {
  query: string;
  response: QueryResponse | null;
  error: string | null;
  isLoading: boolean;
}

export default function Home() {
  const [queryResults, setQueryResults] = useState<QueryResult[]>(
    PLACEHOLDER_QUERIES.map((query) => ({
      query,
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
      const response = await submitQuery({
        query: PLACEHOLDER_QUERIES[index],
        prev_messages: [],
      });

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

  // Run all queries on mount
  useEffect(() => {
    PLACEHOLDER_QUERIES.forEach((_, index) => {
      runQuery(index);
    });
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
                <th className="px-4 py-4 text-left text-sm font-semibold text-gray-900 dark:text-gray-100 w-24">
                  Action
                </th>
                <th className="px-6 py-4 text-left text-sm font-semibold text-gray-900 dark:text-gray-100 w-1/4">
                  Query
                </th>
                <th className="px-6 py-4 text-left text-sm font-semibold text-gray-900 dark:text-gray-100">
                  Result
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
                  <td className="px-4 py-4 align-top">
                    <button
                      onClick={() => runQuery(index)}
                      disabled={item.isLoading}
                      className="text-xs px-3 py-1 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white rounded transition-colors"
                    >
                      {item.isLoading ? 'Loading...' : 'Retry'}
                    </button>
                  </td>
                  <td className="px-6 py-4 align-top">
                    <p className="text-sm text-gray-700 dark:text-gray-300">
                      {item.query}
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
                      <div className="flex gap-4">
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
                            <div className="w-80 space-y-2">
                              <p className="text-xs font-semibold text-gray-600 dark:text-gray-400">
                                Related Documents:
                              </p>
                              <div className="space-y-1">
                                {item.response.ranked_docs.map((doc) => (
                                  <div
                                    key={doc.url}
                                    className="p-2 bg-gray-50 dark:bg-gray-700/50 rounded text-xs"
                                  >
                                    <div className="space-y-1">
                                      <p className="font-medium text-gray-700 dark:text-gray-300 truncate">
                                        {doc.title}
                                      </p>
                                      <a
                                        href={doc.url}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        className="text-blue-600 dark:text-blue-400 hover:underline truncate block"
                                      >
                                        {doc.url}
                                      </a>
                                      <span
                                        className={`inline-block px-2 py-1 rounded text-xs font-medium ${
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
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </main>
    </div>
  );
}
