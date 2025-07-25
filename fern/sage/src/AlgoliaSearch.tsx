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

// Document type icon component using Fern's FontAwesome icons
function DocumentIcon({ type }: { type: string }) {
  const iconStyle = {
    width: '16px',
    height: '16px',
    flexShrink: 0,
    color: '#6b7280',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  };

  const getIconClass = () => {
    switch (type) {
      case 'guide':
        return 'fa-solid fa-book';
      case 'reference':
        return 'fa-solid fa-code';
      case 'example':
        return 'fa-solid fa-grid-2';
      default:
        return 'fa-regular fa-file-lines';
    }
  };

  return (
    <i
      className={getIconClass()}
      style={iconStyle}
      aria-label={`${type} document`}
      title={`${type} document`}
    />
  );
}

// Custom Hit component to display search results with icons and descriptions
function Hit({ hit }: { hit: any }) {
  const [isHovered, setIsHovered] = useState(false);

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

  // Determine document type based on URL path
  const getDocumentType = (pathname: string): string => {
    if (pathname.includes('/guide/')) return 'guide';
    if (pathname.includes('/reference/')) return 'reference';
    if (pathname.includes('/examples/')) return 'example';
    return 'document';
  };

  const documentType = getDocumentType(
    hit.pathname || hit.canonicalPathname || '',
  );

  // Truncate description for display
  const truncateText = (text: string, maxLength: number) => {
    if (text.length <= maxLength) return text;
    return `${text.substring(0, maxLength)}...`;
  };

  const processedDescription = highlightedDescription.replace(/<[^>]*>/g, ''); // Remove HTML tags for length calculation
  const shouldTruncate = processedDescription.length > 120;
  const displayDescription =
    shouldTruncate && !isHovered
      ? truncateText(processedDescription, 120)
      : highlightedDescription;

  return (
    <a
      href={hit.pathname || hit.canonicalPathname || '#'}
      style={{
        display: 'flex',
        alignItems: 'flex-start',
        gap: '12px',
        padding: '12px 16px',
        textDecoration: 'none',
        color: '#374151',
        borderBottom: '1px solid #f3f4f6',
        transition: 'background 0.15s ease',
        cursor: 'pointer',
        background: isHovered ? '#f9fafb' : 'transparent',
      }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      title={shouldTruncate ? processedDescription : undefined}
    >
      {/* Document Icon */}
      <div style={{ paddingTop: '2px' }}>
        <DocumentIcon type={documentType} />
      </div>

      {/* Content */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontWeight: 600,
            fontSize: '14px',
            marginBottom: '4px',
            color: '#111827',
            lineHeight: '1.3',
          }}
        >
          <span
            // eslint-disable-next-line react/no-danger
            dangerouslySetInnerHTML={{
              __html: processHighlights(highlightedTitle),
            }}
          />
        </div>

        {highlightedDescription && (
          <div
            style={{
              fontSize: '13px',
              color: '#6b7280',
              lineHeight: '1.4',
              wordBreak: 'break-word',
            }}
          >
            <span
              // eslint-disable-next-line react/no-danger
              dangerouslySetInnerHTML={{
                __html: processHighlights(displayDescription),
              }}
            />
            {shouldTruncate && !isHovered && (
              <span style={{ color: '#9ca3af', fontStyle: 'italic' }}>
                {' '}
                (hover for full description)
              </span>
            )}
          </div>
        )}

        {hit.breadcrumb && hit.breadcrumb.length > 0 && (
          <div
            style={{
              fontSize: '11px',
              color: '#9ca3af',
              marginTop: '6px',
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
            }}
          >
            <svg
              width="12"
              height="12"
              fill="currentColor"
              viewBox="0 0 20 20"
              aria-label="Breadcrumb"
            >
              <title>Breadcrumb</title>
              <path
                fillRule="evenodd"
                d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z"
                clipRule="evenodd"
              />
            </svg>
            <span>
              {hit.breadcrumb.map((crumb: any, index: number) => (
                <span key={crumb.title}>
                  {index > 0 && ' › '}
                  {crumb.title}
                </span>
              ))}
            </span>
          </div>
        )}
      </div>
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
      e.stopPropagation();
      onClick();
    }
  };

  const handleClick = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    onClick();
  };

  return (
    <button
      type="button"
      onClick={handleClick}
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
function CustomSearchBox({
  onAskAI,
  onToggleAI,
}: {
  onAskAI: (query: string) => void;
  onToggleAI?: () => void;
}) {
  const { query, refine } = useSearchBox();
  const { hits } = useHits();
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

  const handleBlur = (e: React.FocusEvent) => {
    // Only blur if focus is moving outside the search container
    const currentTarget = e.currentTarget;
    setTimeout(() => {
      if (!currentTarget.contains(document.activeElement)) {
        setIsFocused(false);
        setSelectedIndex(-1);
      }
    }, 200); // Increased delay for better UX
  };

  const handleClear = () => {
    setInputValue('');
    refine('');
    setSelectedIndex(-1);
    inputRef.current?.focus();
  };

  const handleAskAI = () => {
    onAskAI(inputValue);
    setIsFocused(false);
  };

  const handleToggleAI = () => {
    if (onToggleAI) {
      onToggleAI();
    }
  };

  // Calculate total selectable items: Ask AI option (when query exists) + search results
  const totalSelectableItems = (inputValue.trim() ? 1 : 0) + hits.length;

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!inputValue.trim() && e.key !== 'Escape') return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((prev) =>
        prev < totalSelectableItems - 1 ? prev + 1 : -1,
      );
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((prev) =>
        prev > -1 ? prev - 1 : totalSelectableItems - 1,
      );
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (selectedIndex === 0 && inputValue.trim()) {
        // Ask AI option is selected
        handleAskAI();
      } else if (selectedIndex > 0) {
        // A search result is selected
        const hitIndex = selectedIndex - 1;
        if (hits[hitIndex]) {
          const hit = hits[hitIndex];
          window.location.href = hit.pathname || hit.canonicalPathname || '#';
        }
      } else if (selectedIndex === -1 && inputValue.trim()) {
        // No selection, trigger Ask AI by default
        handleAskAI();
      }
    } else if (e.key === 'Escape') {
      setIsFocused(false);
      setSelectedIndex(-1);
      inputRef.current?.blur();
    }
  };

  // Handle slash key shortcut
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (
        e.key === '/' &&
        !isFocused &&
        document.activeElement?.tagName !== 'INPUT' &&
        document.activeElement?.tagName !== 'TEXTAREA'
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
            onClick={handleToggleAI}
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
        isFocused={isFocused}
      />
    </div>
  );
}

// Custom Hits component with conditional visibility and Ask AI option
function CustomHits({
  selectedIndex,
  onAskAI,
  query,
  isFocused,
}: {
  selectedIndex?: number;
  onAskAI?: () => void;
  query?: string;
  isFocused?: boolean;
}) {
  const { hits } = useHits();
  const { query: searchQuery } = useSearchBox();
  const [showResults, setShowResults] = useState(false);

  const actualQuery = query || searchQuery;

  // Show results when there's a query and focus, or when there are hits
  useEffect(() => {
    setShowResults(
      (actualQuery.trim().length > 0 && isFocused) || hits.length > 0,
    );
  }, [actualQuery, hits, isFocused]);

  if (!showResults || !isFocused) {
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
      {hits.map((hit: any, index: number) => {
        const adjustedIndex = actualQuery.trim() ? index + 1 : index;
        const isSelected = selectedIndex === adjustedIndex;

        return (
          <div
            key={hit.objectID}
            style={{
              backgroundColor: isSelected ? '#f3f4f6' : 'transparent',
            }}
          >
            <Hit hit={hit} />
          </div>
        );
      })}

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
          {onAskAI && (
            <div style={{ marginTop: '8px' }}>
              <button
                type="button"
                onClick={onAskAI}
                style={{
                  color: '#7c3aed',
                  background: 'none',
                  border: 'none',
                  textDecoration: 'underline',
                  cursor: 'pointer',
                  fontSize: '14px',
                }}
              >
                Ask AI about this instead
              </button>
            </div>
          )}
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
        Search powered by Algolia • Use ↑↓ to navigate, Enter to select, Esc to
        close
      </div>
    </div>
  );
}

export default function AlgoliaSearch({
  onAskAI,
  onToggleAI,
}: {
  onAskAI?: (query: string) => void;
  onToggleAI?: () => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);

  const handleAskAI = (query: string) => {
    if (onAskAI) {
      onAskAI(query);
    }
  };

  const handleToggleAI = () => {
    if (onToggleAI) {
      onToggleAI();
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

        <CustomSearchBox onAskAI={handleAskAI} onToggleAI={handleToggleAI} />
      </InstantSearch>
    </div>
  );
}
