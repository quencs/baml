'use client';

import { liteClient as algoliasearch } from 'algoliasearch/lite';
import type React from 'react';
import { useEffect, useRef, useState } from 'react';
import {
  Configure,
  InstantSearch,
  useHits,
  useSearchBox,
} from 'react-instantsearch';

const SEARCH_INDEX_NAME = 'fern_docs_search';
const APP_ID = 'P6VYURBGG0';
const API_KEY =
  'YWFiYjAzM2UxMGZkODA5YTA1ZTRiODQ3NDU1NzAzZmIwZWRiY2MwMDY1ZDQxMjQzZGE3ZWZhNWFlZDkyYTNjMmZpbHRlcnM9ZG9tYWluJTNBZG9jcy5ib3VuZGFyeW1sLmNvbSUyMEFORCUyMGF1dGhlZCUzQWZhbHNlJTIwQU5EJTIwTk9UJTIwdHlwZSUzQW5hdmlnYXRpb24mcmVzdHJpY3RJbmRpY2VzPWZlcm5fZG9jc19zZWFyY2gmdXNlclRva2VuPWFub255bW91cy11c2VyLWVmNjBiMzU2LTVkMzYtNGU3YS05N2NmLWI5NjMyMDQ3NmQ0NyZ2YWxpZFVudGlsPTE3NTM0ODEwNjE=';

// Create the search client
const searchClient = algoliasearch(APP_ID, API_KEY);

// Create search filters for the domain
const createSearchFilters = () => {
  return 'domain:docs.boundaryml.com AND NOT type:navigation';
};

// Custom Hit component to display search results
function Hit({ hit }: { hit: any }) {
  const processHighlights = (text: string) => {
    return text
      .replace(
        /__ais-highlight__/g,
        '<mark style="background: #fff7a8; color: #ec4899; font-weight: 700; padding: 0 2px; border-radius: 2px;">',
      )
      .replace(/__\/ais-highlight__/g, '</mark>');
  };

  const highlightedTitle =
    hit._highlightResult?.title?.value || hit.title || 'Untitled';
  const highlightedDescription =
    hit._highlightResult?.description?.value ||
    hit._snippetResult?.description?.value ||
    hit.description ||
    '';

  return (
    <a
      href={hit.pathname || hit.canonicalPathname || '#'}
      style={{
        display: 'block',
        padding: '12px 16px',
        textDecoration: 'none',
        color: '#374151',
        borderBottom: '1px solid #f3f4f6',
        transition: 'background 0.15s ease',
        cursor: 'pointer',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.background = '#f9fafb';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.background = 'transparent';
      }}
    >
      <div
        style={{
          fontWeight: 600,
          fontSize: '14px',
          marginBottom: '4px',
          color: '#111827',
        }}
      >
        <span
          dangerouslySetInnerHTML={{
            __html: processHighlights(highlightedTitle),
          }}
        />
      </div>
      {highlightedDescription && (
        <div style={{ fontSize: '13px', color: '#6b7280', lineHeight: '1.4' }}>
          <span
            dangerouslySetInnerHTML={{
              __html: processHighlights(highlightedDescription),
            }}
          />
        </div>
      )}
      {hit.breadcrumb && hit.breadcrumb.length > 0 && (
        <div style={{ fontSize: '11px', color: '#9ca3af', marginTop: '6px' }}>
          {hit.breadcrumb.map((crumb: any, index: number) => (
            <span key={crumb.title}>
              {index > 0 && ' › '}
              {crumb.title}
            </span>
          ))}
        </div>
      )}
    </a>
  );
}

// Search Icon Component
function SearchIcon() {
  return (
    <svg
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-label="Search"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
      />
    </svg>
  );
}

// Slash Icon Component
function SlashIcon() {
  return (
    <div
      style={{
        padding: '2px 6px',
        background: '#f3f4f6',
        borderRadius: '4px',
        fontSize: '11px',
        fontWeight: 600,
        color: '#6b7280',
        fontFamily: 'monospace',
      }}
    >
      /
    </div>
  );
}

// AI Icon Component
function AIIcon() {
  return (
    <svg
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-label="AI"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
      />
    </svg>
  );
}

// Ask with AI component
function AskWithAIOption({
  isSelected,
  onClick,
  query,
}: {
  isSelected: boolean;
  onClick: () => void;
  query: string;
}) {
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onClick();
    }
  };

  return (
    <button
      type="button"
      onClick={onClick}
      onKeyDown={handleKeyDown}
      style={{
        display: 'block',
        width: '100%',
        padding: '12px 16px',
        textDecoration: 'none',
        color: '#374151',
        borderBottom: '1px solid #f3f4f6',
        border: 'none',
        textAlign: 'left',
        transition: 'background 0.15s ease',
        cursor: 'pointer',
        background: isSelected ? '#f3f4f6' : 'transparent',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.background = '#f3f4f6';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.background = isSelected
          ? '#f3f4f6'
          : 'transparent';
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          fontWeight: 600,
          fontSize: '14px',
          color: '#7c3aed',
        }}
      >
        <AIIcon />
        Ask with AI about "{query}"
      </div>
      <div
        style={{
          fontSize: '13px',
          color: '#6b7280',
          lineHeight: '1.4',
          marginTop: '4px',
        }}
      >
        Get AI-powered insights about your search query
      </div>
    </button>
  );
}

// Custom SearchBox with integrated controls
function CustomSearchBox({ onAskAI }: { onAskAI: (query: string) => void }) {
  const { query, refine } = useSearchBox();
  const [inputValue, setInputValue] = useState(query);
  const [isFocused, setIsFocused] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setInputValue(query);
  }, [query]);

  const handleInputChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    setInputValue(value);
    refine(value);
    setSelectedIndex(-1); // Reset selection when typing
  };

  const handleFocus = () => {
    setIsFocused(true);
  };

  const handleBlur = () => {
    // Delay blur to allow clicking on results
    setTimeout(() => {
      setIsFocused(false);
      setSelectedIndex(-1);
    }, 150);
  };

  const handleClear = () => {
    setInputValue('');
    refine('');
    setSelectedIndex(-1);
    inputRef.current?.focus();
  };

  const handleAskAI = () => {
    onAskAI(inputValue);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((prev) => Math.min(prev + 1, 0)); // Only Ask AI option for now
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((prev) => Math.max(prev - 1, -1));
    } else if (e.key === 'Enter' && selectedIndex === 0) {
      e.preventDefault();
      handleAskAI();
    }
  };

  // Handle slash key shortcut
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (
        e.key === '/' &&
        !isFocused &&
        document.activeElement?.tagName !== 'INPUT'
      ) {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };

    document.addEventListener('keydown', handleGlobalKeyDown);
    return () => document.removeEventListener('keydown', handleGlobalKeyDown);
  }, [isFocused]);

  return (
    <div style={{ position: 'relative', width: '100%' }}>
      <div
        style={{
          position: 'relative',
          display: 'flex',
          alignItems: 'center',
          background: '#ffffff',
          border: `1.5px solid ${isFocused ? '#7c3aed' : '#e5e7eb'}`,
          borderRadius: '10px',
          transition: 'border-color 0.2s ease, box-shadow 0.2s ease',
          boxShadow: isFocused ? '0 0 0 3px rgba(124, 58, 237, 0.1)' : 'none',
        }}
      >
        {/* Search Icon */}
        <div
          style={{
            position: 'absolute',
            left: '12px',
            color: '#9ca3af',
            display: 'flex',
            alignItems: 'center',
          }}
        >
          <SearchIcon />
        </div>

        {/* Input Field */}
        <input
          ref={inputRef}
          type="text"
          value={inputValue}
          onChange={handleInputChange}
          onFocus={handleFocus}
          onBlur={handleBlur}
          onKeyDown={handleKeyDown}
          placeholder="Search BAML docs…"
          style={{
            flex: 1,
            padding: '12px 160px 12px 40px',
            border: 'none',
            outline: 'none',
            background: 'transparent',
            fontSize: '14px',
            color: '#111827',
            fontFamily: 'inherit',
          }}
        />

        {/* Right side controls */}
        <div
          style={{
            position: 'absolute',
            right: '8px',
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
          }}
        >
          {/* Clear button */}
          {inputValue && (
            <button
              type="button"
              onClick={handleClear}
              style={{
                padding: '4px',
                background: 'none',
                border: 'none',
                cursor: 'pointer',
                color: '#9ca3af',
                borderRadius: '4px',
                display: 'flex',
                alignItems: 'center',
                fontSize: '14px',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.color = '#6b7280';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.color = '#9ca3af';
              }}
            >
              ✕
            </button>
          )}

          {/* Ask AI button */}
          <button
            type="button"
            onClick={handleAskAI}
            style={{
              padding: '6px 10px',
              background: '#7c3aed',
              border: 'none',
              borderRadius: '6px',
              color: 'white',
              fontSize: '12px',
              fontWeight: 600,
              cursor: 'pointer',
              transition: 'background 0.2s ease',
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = '#6d28d9';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = '#7c3aed';
            }}
          >
            Ask AI
          </button>

          {/* Slash shortcut indicator */}
          {!isFocused && !inputValue && <SlashIcon />}
        </div>
      </div>

      {/* Pass selectedIndex and handlers to hits */}
      <CustomHits
        selectedIndex={selectedIndex}
        onAskAI={() => handleAskAI()}
        query={inputValue}
      />
    </div>
  );
}

// Custom Hits component with conditional visibility and Ask AI option
function CustomHits({
  selectedIndex,
  onAskAI,
  query,
}: {
  selectedIndex?: number;
  onAskAI?: () => void;
  query?: string;
}) {
  const { hits } = useHits();
  const { query: searchQuery } = useSearchBox();
  const [showResults, setShowResults] = useState(false);

  const actualQuery = query || searchQuery;

  // Only show results when there's a query and hits
  useEffect(() => {
    setShowResults(actualQuery.trim().length > 0);
  }, [actualQuery, hits]);

  if (!showResults) {
    return null;
  }

  return (
    <div
      style={{
        position: 'absolute',
        top: '100%',
        left: 0,
        right: 0,
        marginTop: '4px',
        background: '#ffffff',
        border: '1px solid #e5e7eb',
        borderRadius: '10px',
        boxShadow:
          '0 10px 25px rgba(0, 0, 0, 0.1), 0 4px 6px rgba(0, 0, 0, 0.05)',
        zIndex: 1000,
        maxHeight: '400px',
        overflowY: 'auto',
        overflowX: 'hidden',
      }}
    >
      {/* Ask with AI option */}
      {onAskAI && actualQuery.trim() && (
        <AskWithAIOption
          isSelected={selectedIndex === 0}
          onClick={onAskAI}
          query={actualQuery}
        />
      )}

      {/* Search results */}
      {hits.map((hit: any) => (
        <Hit key={hit.objectID} hit={hit} />
      ))}

      {/* No results message when there are no hits but there's a query */}
      {hits.length === 0 && actualQuery.trim() && (
        <div
          style={{
            padding: '16px',
            textAlign: 'center',
            color: '#6b7280',
            fontSize: '14px',
          }}
        >
          No results found for "{actualQuery}"
        </div>
      )}

      {/* Footer */}
      <div
        style={{
          padding: '8px 16px',
          borderTop: '1px solid #f3f4f6',
          background: '#f9fafb',
          fontSize: '11px',
          color: '#6b7280',
          textAlign: 'center',
        }}
      >
        Search powered by Algolia
      </div>
    </div>
  );
}

export default function AlgoliaSearch({
  onAskAI,
}: { onAskAI?: (query: string) => void }) {
  const containerRef = useRef<HTMLDivElement>(null);

  const handleAskAI = (query: string) => {
    if (onAskAI) {
      onAskAI(query);
    }
  };

  return (
    <div ref={containerRef} style={{ position: 'relative', width: '100%' }}>
      <InstantSearch
        indexName={SEARCH_INDEX_NAME}
        searchClient={searchClient}
        future={{
          preserveSharedStateOnUnmount: true,
        }}
      >
        <Configure
          filters={createSearchFilters()}
          hitsPerPage={8}
          attributesToHighlight={['title', 'description', 'content']}
          attributesToSnippet={['description:50', 'content:50']}
          highlightPreTag="__ais-highlight__"
          highlightPostTag="__/ais-highlight__"
          distinct={true}
          analytics={true}
          analyticsTags={['desktop', 'docs.boundaryml.com', 'search-v2-dialog']}
        />

        <CustomSearchBox onAskAI={handleAskAI} />
        <CustomHits />
      </InstantSearch>
    </div>
  );
}
