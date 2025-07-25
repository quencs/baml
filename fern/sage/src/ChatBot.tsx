import { useAtom } from 'jotai';
import React, { useRef, useEffect, useState } from 'react';
import BamlLambWhite from './baml-lamb-white.svg';
import { type Message, messagesAtom } from './store';

interface ChatBotProps {
  apiEndpoint?: string;
  isOpen?: boolean;
  onClose?: () => void;
}

const API_ENDPOINT =
  process.env.NODE_ENV === 'development'
    ? 'http://localhost:4000/api/doc-chat'
    : 'https://baml-sage-backend.vercel.app/api/doc-chat';

const ChatBot: React.FC<ChatBotProps> = ({ isOpen = false, onClose }) => {
  const [messages, setMessages] = useAtom(messagesAtom);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [pendingQuery, setPendingQuery] = useState<string | null>(null);
  // Add width state for resizing
  const [width, setWidth] = useState(400);
  const [isResizing, setIsResizing] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  // Check for stored AI query on mount and when opening, and periodically when open
  useEffect(() => {
    if (!isOpen) return;

    const checkForNewQuery = () => {
      try {
        const storedContext = localStorage.getItem('baml-ai-context');
        if (storedContext) {
          const context = JSON.parse(storedContext);
          const now = Date.now();

          // Check if the context is recent (within 10 seconds)
          if (context.query && now - context.timestamp < 10000) {
            // Set the pending query to be sent
            setPendingQuery(context.query);

            // Clear the stored context after using it
            localStorage.removeItem('baml-ai-context');
          }
        }
      } catch (error) {
        console.error('Error processing stored AI context:', error);
        localStorage.removeItem('baml-ai-context');
      }
    };

    // Check immediately when opening
    checkForNewQuery();

    // Set up interval to check for new queries while panel is open
    const interval = setInterval(checkForNewQuery, 100);

    return () => {
      clearInterval(interval);
    };
  }, [isOpen]);

  // Clear chat functionality
  const clearChat = () => {
    setMessages([]);
  };

  const sendMessage = async (text: string, retryMessageId?: string) => {
    if (!text.trim()) return;

    let messagesWithUser: Message[];

    if (retryMessageId) {
      // Find and update the existing error message
      messagesWithUser = messages.map((msg) =>
        msg.id === retryMessageId
          ? { ...msg, error: false, text: '...thinking' }
          : msg,
      );
      setMessages(messagesWithUser);
    } else {
      const userMessage: Message = {
        id: Date.now().toString(),
        text: text.trim(),
        isUser: true,
        timestamp: new Date(),
      };
      messagesWithUser = [...messages, userMessage];
      setMessages(messagesWithUser);
      setInput('');
    }

    setIsLoading(true);

    try {
      const response = await fetch(API_ENDPOINT, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        // TODO: attach old messages to the context
        body: JSON.stringify({ query: text.trim() }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      let data: any;
      try {
        data = await response.json();
      } catch (jsonError) {
        console.error('Failed to parse JSON response:', jsonError);
        throw new Error('Invalid JSON response from server');
      }

      // Validate that we have the expected response structure
      if (!data || typeof data.answer !== 'string') {
        console.error('Invalid response structure:', data);
        throw new Error('Invalid response structure from AI service');
      }

      const botMessage: Message = {
        id: retryMessageId || (Date.now() + 1).toString(),
        text: data.answer || "Sorry, I couldn't process your request.",
        isUser: false,
        timestamp: new Date(),
        ranked_docs: data.ranked_docs,
      };

      if (retryMessageId) {
        // Update the existing message
        setMessages(
          messagesWithUser.map((msg) =>
            msg.id === retryMessageId ? botMessage : msg,
          ),
        );
      } else {
        setMessages([...messagesWithUser, botMessage]);
      }

      // Auto-navigate to first ranked doc if available
      if (data.ranked_docs && data.ranked_docs.length > 0) {
        const firstDoc = data.ranked_docs[0];
        // Use the global navigateToDoc function
        if ((window as any).navigateToDoc) {
          (window as any).navigateToDoc(
            { u: firstDoc.url, t: firstDoc.title, sel: 'article' },
            text.trim(),
          );
        }
      }
    } catch (error) {
      console.error('Error sending message:', error);

      // Determine more specific error message
      let errorText = 'Sorry, there was an error processing your request.';
      if (error instanceof TypeError && error.message.includes('fetch')) {
        errorText =
          'Unable to connect to the AI service. Please check your connection and try again.';
      } else if (error instanceof Error) {
        if (error.message.includes('HTTP error! status: 429')) {
          errorText = 'Too many requests. Please wait a moment and try again.';
        } else if (error.message.includes('HTTP error! status: 500')) {
          errorText =
            'Server error occurred. The AI service may be temporarily unavailable.';
        } else if (error.message.includes('HTTP error! status: 404')) {
          errorText =
            'AI service endpoint not found. Please check the configuration.';
        } else if (error.message.includes('HTTP error!')) {
          errorText = `Service error: ${error.message}. Please try again.`;
        } else if (error.message.includes('JSON')) {
          errorText =
            'Received invalid response from AI service. Please try again.';
        }
      }

      const errorMessage: Message = {
        id: retryMessageId || (Date.now() + 1).toString(),
        text: errorText,
        isUser: false,
        timestamp: new Date(),
        error: true,
        originalQuery: text.trim(),
      };

      if (retryMessageId) {
        // Update the existing message
        setMessages(
          messagesWithUser.map((msg) =>
            msg.id === retryMessageId ? errorMessage : msg,
          ),
        );
      } else {
        setMessages([...messagesWithUser, errorMessage]);
      }
    } finally {
      setIsLoading(false);
    }
  };

  // Handle pending query from Ask Baaaaml functionality
  useEffect(() => {
    if (pendingQuery && !isLoading) {
      sendMessage(pendingQuery);
      setPendingQuery(null);
    }
  }, [pendingQuery, isLoading]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    sendMessage(input);
  };

  const handleRetry = (message: Message) => {
    if (message.originalQuery) {
      sendMessage(message.originalQuery, message.id);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage(input);
    }
  };

  const handleClose = () => {
    if (onClose) {
      onClose();
    }
  };

  // Add resize handlers
  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  };

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing) return;

      const newWidth = window.innerWidth - e.clientX;
      // Set min and max width constraints
      const minWidth = 300;
      const maxWidth = Math.min(800, window.innerWidth * 0.8);

      if (newWidth >= minWidth && newWidth <= maxWidth) {
        setWidth(newWidth);
      }
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    if (isResizing) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = 'ew-resize';
      document.body.style.userSelect = 'none';
    }

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, [isResizing]);

  // Calculate panel position below header
  const [panelTop, setPanelTop] = React.useState(0);
  const [panelHeight, setPanelHeight] = React.useState('100vh');

  React.useEffect(() => {
    const updatePosition = () => {
      const header = document.querySelector(
        'header, .fern-header',
      ) as HTMLElement;
      const top = header ? header.getBoundingClientRect().bottom : 0;
      setPanelTop(top);
      setPanelHeight(`calc(100vh - ${top}px)`);
    };

    updatePosition();
    window.addEventListener('resize', updatePosition);
    window.addEventListener('scroll', updatePosition, { passive: true });

    return () => {
      window.removeEventListener('resize', updatePosition);
      window.removeEventListener('scroll', updatePosition);
    };
  }, []);

  return (
    <div
      style={{
        position: 'fixed',
        top: `${panelTop}px`,
        right: '0',
        width: `${width}px`,
        height: panelHeight,
        backgroundColor: '#ffffff',
        borderLeft: '1px solid #e5e7eb',
        transform: isOpen ? 'translateX(0)' : 'translateX(100%)',
        transition: isResizing
          ? 'none'
          : 'transform .3s cubic-bezier(.4,0,.2,1)',
        display: 'flex',
        flexDirection: 'column',
        zIndex: 2000,
        fontFamily:
          'Inter, system-ui, -apple-system, BlinkMacSystemFont, sans-serif',
        overflow: 'hidden',
      }}
    >
      {/* Resize Handle */}
      <div
        onMouseDown={handleMouseDown}
        style={{
          position: 'absolute',
          left: '0',
          top: '0',
          bottom: '0',
          width: '4px',
          cursor: 'ew-resize',
          backgroundColor: 'transparent',
          zIndex: 10,
          transition: 'background-color 0.2s ease',
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.backgroundColor = '#667eea';
        }}
        onMouseLeave={(e) => {
          if (!isResizing) {
            e.currentTarget.style.backgroundColor = 'transparent';
          }
        }}
      />

      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          height: '64px',
          padding: '0 24px',
          fontSize: '16px',
          fontWeight: '700',
          backgroundColor: '#ffffff',
          color: '#111827',
          borderBottom: '1px solid #e5e7eb',
          position: 'relative',
          zIndex: 1,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {/* BAML Icon */}
          <div
            style={{
              width: '28px',
              height: '28px',
              borderRadius: '6px',
              background: '#667eea',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '2px',
            }}
          >
            <img
              src={BamlLambWhite}
              alt="BAML Logo"
              style={{
                width: '26px',
                height: '26px',
                filter: 'drop-shadow(0 0 1px rgba(255,255,255,0.8))',
              }}
            />
          </div>
          <span style={{ color: '#111827' }}>BAML Assistant</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {/* Clear Chat Button */}
          {messages.length > 0 && (
            <button
              type="button"
              onClick={clearChat}
              style={{
                background: 'none',
                border: '1px solid #e5e7eb',
                fontSize: '12px',
                color: '#6b7280',
                cursor: 'pointer',
                padding: '6px 12px',
                borderRadius: '6px',
                fontWeight: '500',
                transition: 'all 0.2s ease',
              }}
              onMouseOver={(e) => {
                e.currentTarget.style.backgroundColor = '#f9fafb';
                e.currentTarget.style.borderColor = '#d1d5db';
                e.currentTarget.style.color = '#374151';
              }}
              onFocus={(e) => {
                e.currentTarget.style.backgroundColor = '#f9fafb';
                e.currentTarget.style.borderColor = '#d1d5db';
                e.currentTarget.style.color = '#374151';
              }}
              onMouseOut={(e) => {
                e.currentTarget.style.backgroundColor = 'transparent';
                e.currentTarget.style.borderColor = '#e5e7eb';
                e.currentTarget.style.color = '#6b7280';
              }}
              onBlur={(e) => {
                e.currentTarget.style.backgroundColor = 'transparent';
                e.currentTarget.style.borderColor = '#e5e7eb';
                e.currentTarget.style.color = '#6b7280';
              }}
            >
              Clear
            </button>
          )}
          {/* Close Button */}
          <button
            type="button"
            onClick={handleClose}
            style={{
              background: 'none',
              border: 'none',
              fontSize: '20px',
              color: '#6b7280',
              cursor: 'pointer',
              opacity: 0.7,
              padding: '4px',
              lineHeight: 1,
              transition: 'all 0.2s ease',
              borderRadius: '4px',
            }}
            onMouseOver={(e) => {
              e.currentTarget.style.opacity = '1';
              e.currentTarget.style.backgroundColor = '#f3f4f6';
            }}
            onFocus={(e) => {
              e.currentTarget.style.opacity = '1';
              e.currentTarget.style.backgroundColor = '#f3f4f6';
            }}
            onMouseOut={(e) => {
              e.currentTarget.style.opacity = '0.7';
              e.currentTarget.style.backgroundColor = 'transparent';
            }}
            onBlur={(e) => {
              e.currentTarget.style.opacity = '0.7';
              e.currentTarget.style.backgroundColor = 'transparent';
            }}
          >
            ✕
          </button>
        </div>
      </div>

      {/* Messages */}
      <main
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '24px',
          display: 'flex',
          flexDirection: 'column',
          backgroundColor: '#fafafa',
          gap: '16px',
        }}
        className="chat-scrollbar"
      >
        {messages.length === 0 && (
          <div
            style={{
              textAlign: 'center',
              color: '#6b7280',
              fontStyle: 'normal',
              marginTop: '40px',
              padding: '24px',
              backgroundColor: '#ffffff',
              borderRadius: '12px',
              border: '1px solid #e5e7eb',
            }}
          >
            <div style={{ fontSize: '24px', marginBottom: '8px' }}>👋</div>
            <div
              style={{
                fontWeight: '600',
                marginBottom: '4px',
                color: '#111827',
              }}
            >
              Welcome to BAML Assistant
            </div>
            <div style={{ fontSize: '14px', lineHeight: '1.5' }}>
              I'm here to help you with the BAML documentation. Ask me anything
              about functions, types, clients, or examples!
            </div>
          </div>
        )}

        {messages.map((message) => (
          <div
            key={message.id}
            style={{
              alignSelf: message.isUser ? 'flex-end' : 'flex-start',
              maxWidth: '85%',
              display: 'flex',
              flexDirection: 'column',
              gap: '8px',
            }}
          >
            <div
              style={{
                padding: '12px 16px',
                borderRadius: message.isUser
                  ? '16px 16px 4px 16px'
                  : '16px 16px 16px 4px',
                fontSize: '14px',
                lineHeight: '1.5',
                backgroundColor: message.isUser
                  ? '#667eea'
                  : message.error
                    ? '#fef2f2'
                    : '#ffffff',
                color: message.isUser
                  ? '#ffffff'
                  : message.error
                    ? '#dc2626'
                    : '#111827',
                wordWrap: 'break-word',
                border: message.isUser ? 'none' : '1px solid #e5e7eb',
                boxShadow: message.isUser
                  ? '0 4px 12px rgba(102, 126, 234, 0.3)'
                  : '0 2px 8px rgba(0, 0, 0, 0.08)',
              }}
            >
              {message.text}
              {message.error && (
                <div style={{ marginTop: '12px' }}>
                  <button
                    type="button"
                    onClick={() => handleRetry(message)}
                    style={{
                      padding: '8px 16px',
                      fontSize: '13px',
                      backgroundColor: '#6366f1',
                      color: '#fff',
                      border: 'none',
                      borderRadius: '8px',
                      cursor: 'pointer',
                      fontWeight: '500',
                      transition: 'all 0.2s ease',
                      display: 'flex',
                      alignItems: 'center',
                      gap: '6px',
                    }}
                    onMouseOver={(e) => {
                      e.currentTarget.style.backgroundColor = '#5d68e4';
                      e.currentTarget.style.transform = 'translateY(-1px)';
                    }}
                    onFocus={(e) => {
                      e.currentTarget.style.backgroundColor = '#5d68e4';
                      e.currentTarget.style.transform = 'translateY(-1px)';
                    }}
                    onMouseOut={(e) => {
                      e.currentTarget.style.backgroundColor = '#6366f1';
                      e.currentTarget.style.transform = 'translateY(0)';
                    }}
                    onBlur={(e) => {
                      e.currentTarget.style.backgroundColor = '#6366f1';
                      e.currentTarget.style.transform = 'translateY(0)';
                    }}
                  >
                    <svg
                      width="14"
                      height="14"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                      aria-label="Retry"
                    >
                      <title>Retry Icon</title>
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                      />
                    </svg>
                    Try Again
                  </button>
                </div>
              )}
            </div>

            {/* Related docs */}
            {message.ranked_docs && message.ranked_docs.length > 0 && (
              <div
                style={{
                  fontSize: '12px',
                  color: '#6b7280',
                  padding: '12px',
                  backgroundColor: '#ffffff',
                  borderRadius: '8px',
                  border: '1px solid #e5e7eb',
                }}
              >
                <div
                  style={{
                    fontWeight: '600',
                    marginBottom: '8px',
                    color: '#374151',
                  }}
                >
                  📖 Related documentation:
                </div>
                {message.ranked_docs.map((doc) => (
                  <div key={doc.url} style={{ marginBottom: '4px' }}>
                    <a
                      href={doc.url}
                      style={{
                        color: '#667eea',
                        textDecoration: 'none',
                        fontSize: '12px',
                        transition: 'all 0.2s ease',
                        fontWeight: '500',
                      }}
                      onMouseOver={(e) => {
                        e.currentTarget.style.textDecoration = 'underline';
                        e.currentTarget.style.color = '#5a67d8';
                      }}
                      onFocus={(e) => {
                        e.currentTarget.style.textDecoration = 'underline';
                        e.currentTarget.style.color = '#5a67d8';
                      }}
                      onMouseOut={(e) => {
                        e.currentTarget.style.textDecoration = 'none';
                        e.currentTarget.style.color = '#667eea';
                      }}
                      onBlur={(e) => {
                        e.currentTarget.style.textDecoration = 'none';
                        e.currentTarget.style.color = '#667eea';
                      }}
                    >
                      {doc.title}
                    </a>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}

        {isLoading && (
          <div
            style={{
              maxWidth: '85%',
              padding: '12px 16px',
              borderRadius: '16px 16px 16px 4px',
              fontSize: '14px',
              lineHeight: '1.5',
              alignSelf: 'flex-start',
              backgroundColor: '#ffffff',
              color: '#6b7280',
              border: '1px solid #e5e7eb',
              boxShadow: '0 2px 8px rgba(0, 0, 0, 0.08)',
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
            }}
          >
            <div
              style={{
                width: '6px',
                height: '6px',
                borderRadius: '50%',
                backgroundColor: '#667eea',
                animation: 'pulse 1.5s ease-in-out infinite',
              }}
            />
            <div
              style={{
                width: '6px',
                height: '6px',
                borderRadius: '50%',
                backgroundColor: '#667eea',
                animation: 'pulse 1.5s ease-in-out infinite 0.2s',
              }}
            />
            <div
              style={{
                width: '6px',
                height: '6px',
                borderRadius: '50%',
                backgroundColor: '#667eea',
                animation: 'pulse 1.5s ease-in-out infinite 0.4s',
              }}
            />
            <span>Thinking...</span>
          </div>
        )}

        <div ref={messagesEndRef} />
      </main>

      {/* Input Form */}
      <form
        onSubmit={handleSubmit}
        style={{
          display: 'flex',
          padding: '16px 24px',
          borderTop: '1px solid #e5e7eb',
          backgroundColor: '#ffffff',
          gap: '12px',
          alignItems: 'flex-end',
        }}
      >
        <div style={{ flex: 1, position: 'relative' }}>
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                sendMessage(input);
              }
            }}
            placeholder="Ask me anything about BAML..."
            disabled={isLoading}
            rows={1}
            style={{
              width: '100%',
              minHeight: '44px',
              maxHeight: '120px',
              padding: '12px 16px',
              border: '1px solid #e5e7eb',
              borderRadius: '12px',
              fontSize: '14px',
              outline: 'none',
              fontFamily: 'inherit',
              backgroundColor: '#ffffff',
              color: '#111827',
              resize: 'none',
              transition: 'border-color 0.2s ease',
            }}
            onFocus={(e) => {
              e.currentTarget.style.borderColor = '#667eea';
            }}
            onBlur={(e) => {
              e.currentTarget.style.borderColor = '#e5e7eb';
            }}
          />
        </div>
        <button
          type="submit"
          disabled={isLoading || !input.trim()}
          style={{
            border: 'none',
            padding: '12px 20px',
            background: input.trim() ? '#667eea' : '#e5e7eb',
            color: input.trim() ? '#ffffff' : '#9ca3af',
            fontWeight: '600',
            cursor: isLoading || !input.trim() ? 'not-allowed' : 'pointer',
            borderRadius: '12px',
            fontSize: '14px',
            transition: 'all 0.2s ease',
            minWidth: '64px',
          }}
          onMouseOver={(e) => {
            if (input.trim() && !isLoading) {
              e.currentTarget.style.backgroundColor = '#5a67d8';
            }
          }}
          onFocus={(e) => {
            if (input.trim() && !isLoading) {
              e.currentTarget.style.backgroundColor = '#5a67d8';
            }
          }}
          onMouseOut={(e) => {
            if (input.trim() && !isLoading) {
              e.currentTarget.style.backgroundColor = '#667eea';
            }
          }}
          onBlur={(e) => {
            if (input.trim() && !isLoading) {
              e.currentTarget.style.backgroundColor = '#667eea';
            }
          }}
        >
          Send
        </button>
      </form>

      <style>
        {`
          @keyframes pulse {
            0%, 100% { opacity: 0.4; transform: scale(0.8); }
            50% { opacity: 1; transform: scale(1); }
          }
          
          /* Improved scrollbar styling for chat */
          .chat-scrollbar::-webkit-scrollbar {
            width: 6px;
          }
          .chat-scrollbar::-webkit-scrollbar-track {
            background: transparent;
          }
          .chat-scrollbar::-webkit-scrollbar-thumb {
            background: #d1d5db;
            border-radius: 3px;
          }
          .chat-scrollbar::-webkit-scrollbar-thumb:hover {
            background: #9ca3af;
          }
        `}
      </style>
    </div>
  );
};

export default ChatBot;
