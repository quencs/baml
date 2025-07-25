import React, { useRef, useEffect, useState } from 'react';
import { useAtom } from 'jotai';
import { messagesAtom, type Message } from './store';

interface ChatBotProps {
  apiEndpoint?: string;
  isOpen?: boolean;
  onClose?: () => void;
}

const API_ENDPOINT = 'http://localhost:4000/api/doc-chat';

const ChatBot: React.FC<ChatBotProps> = ({ isOpen = false, onClose }) => {
  const [messages, setMessages] = useAtom(messagesAtom);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

<<<<<<< HEAD
  // Check for AI context when the chatbot opens
  useEffect(() => {
    if (isOpen && messages.length === 0) {
      const aiContext = localStorage.getItem('baml-ai-context');
      if (aiContext) {
        try {
          const context = JSON.parse(aiContext);
          const timeDiff = Date.now() - context.timestamp;

          // Only use context if it's recent (within 10 seconds)
          if (timeDiff < 10000) {
            localStorage.removeItem('baml-ai-context');

            // Add the user's question
            const userMessage: Message = {
              id: Date.now().toString(),
              text: context.query,
              isUser: true,
              timestamp: new Date(),
            };

            // Generate a placeholder AI response based on the query and page
            const placeholderResponse = generatePlaceholderResponse(
              context.query,
              context.suggestedPage,
            );

            const aiMessage: Message = {
              id: (Date.now() + 1).toString(),
              text: placeholderResponse,
              isUser: false,
              timestamp: new Date(),
            };

            setMessages([userMessage, aiMessage]);
          }
        } catch (error) {
          console.error('Error parsing AI context:', error);
          localStorage.removeItem('baml-ai-context');
        }
      }
    }
  }, [isOpen, messages.length]);

  const generatePlaceholderResponse = (
    query: string,
    suggestedPage: string,
  ): string => {
    const responses: Record<string, string> = {
      '/docs/guide/languages/typescript': `Great question about "${query}"! I've navigated you to the TypeScript guide which covers how to use BAML with TypeScript. Here you'll find information about generating TypeScript clients, type safety, and integration patterns. The TypeScript client provides excellent IntelliSense and compile-time checking for your BAML functions.`,
      '/docs/guide/languages/python': `Perfect! For "${query}", I've taken you to the Python documentation. This page explains how to integrate BAML with Python applications, including how to install the Python client, call BAML functions, and handle responses. Python is one of the most popular languages for AI applications with BAML.`,
      '/docs/guide/baml-basics/functions': `Excellent question about "${query}"! I've navigated to the Functions guide which explains how to define and use BAML functions. This is the core of BAML - where you define your AI function signatures, prompts, and expected outputs. You'll learn about function syntax, parameters, and return types.`,
      '/docs/guide/baml-basics/clients': `Great query about "${query}"! I've directed you to the Clients documentation. This covers how to configure different LLM providers (OpenAI, Claude, etc.), set up authentication, and manage multiple model configurations. Clients are how BAML connects to various AI services.`,
      '/docs/guide/prompt-engineering/overview': `Fantastic question about "${query}"! I've taken you to the Prompt Engineering section. This is crucial for getting the best results from your AI functions. You'll learn about prompt optimization, few-shot examples, and best practices for crafting effective prompts.`,
      '/docs/guide/development/testing': `Perfect question about "${query}"! I've navigated to the Testing guide. Testing AI functions is important for reliability - this page covers how to write tests for your BAML functions, mock responses, and ensure consistent behavior across different models.`,
    };

    return (
      responses[suggestedPage] ||
      `Thanks for asking about "${query}"! I've found a relevant page that should help answer your question. This documentation section contains detailed information about the topic you're interested in. Feel free to ask me more specific questions about what you find here!`
    );
  };

  const sendMessage = async (text: string) => {
||||||| parent of aae45cc07 (add more build steps)
  const sendMessage = async (text: string) => {
=======
  const sendMessage = async (text: string, retryMessageId?: string) => {
>>>>>>> aae45cc07 (add more build steps)
    if (!text.trim()) return;

    let messagesWithUser: Message[];
    
    if (retryMessageId) {
      // Find and update the existing error message
      messagesWithUser = messages.map(msg => 
        msg.id === retryMessageId 
          ? { ...msg, error: false, text: '...thinking' }
          : msg
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

      const data = await response.json();

      const botMessage: Message = {
        id: retryMessageId || (Date.now() + 1).toString(),
        text: data.answer || "Sorry, I couldn't process your request.",
        isUser: false,
        timestamp: new Date(),
        ranked_docs: data.ranked_docs,
      };

      if (retryMessageId) {
        // Update the existing message
        setMessages(messagesWithUser.map(msg => 
          msg.id === retryMessageId ? botMessage : msg
        ));
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
            text.trim()
          );
        }
      }
    } catch (error) {
      console.error('Error sending message:', error);
      const errorMessage: Message = {
        id: retryMessageId || (Date.now() + 1).toString(),
        text: 'Sorry, there was an error processing your request.',
        isUser: false,
        timestamp: new Date(),
        error: true,
        originalQuery: text.trim(),
      };
      
      if (retryMessageId) {
        // Update the existing message
        setMessages(messagesWithUser.map(msg => 
          msg.id === retryMessageId ? errorMessage : msg
        ));
      } else {
        setMessages([...messagesWithUser, errorMessage]);
      }
    } finally {
      setIsLoading(false);
    }
  };

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

  // Always render the panel for smooth animations

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
        width: '380px',
        height: panelHeight,
        backgroundColor: 'var(--background)',
        borderLeft: '1px solid var(--border)',
        transform: isOpen ? 'translateX(0)' : 'translateX(100%)',
        transition: 'transform .3s cubic-bezier(.4,0,.2,1)',
        display: 'flex',
        flexDirection: 'column',
        zIndex: 2000,
        fontFamily:
          'Inter, system-ui, -apple-system, BlinkMacSystemFont, sans-serif',
        overflow: 'hidden',
      }}
    >
      {/* Background gradient overlay */}
      <div
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: 'linear-gradient(180deg, rgba(96, 37, 209, 0.15) 0%, rgba(96, 37, 209, 0.05) 20%, rgba(0, 0, 0, 0) 40%)',
          pointerEvents: 'none',
          zIndex: -1,
        }}
        className="chatbot-gradient"
      />
      {/* Pattern overlay */}
      <div
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          opacity: 0.05,
          backgroundSize: '60px 60px',
          maskImage: 'linear-gradient(to bottom, black 0%, transparent 40%)',
          WebkitMaskImage: 'linear-gradient(to bottom, black 0%, transparent 40%)',
          pointerEvents: 'none',
          zIndex: -1,
        }}
        className="chatbot-pattern"
      />
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          height: '56px',
          padding: '0 20px',
          fontSize: '15px',
          fontWeight: '600',
          backgroundColor: 'var(--tag-primary)',
          color: 'var(--accent-primary)',
          borderBottom: '1px solid var(--border)',
          position: 'relative',
          zIndex: 1,
        }}
      >
        <span>BAML AI</span>
        <button
          type="button"
          onClick={handleClose}
          style={{
            background: 'none',
            border: 'none',
            fontSize: '26px',
            color: 'var(--text)',
            cursor: 'pointer',
            opacity: 0.7,
            padding: '0',
            lineHeight: 1,
            transition: 'opacity 0.2s ease',
          }}
          onMouseOver={(e) => {
            e.currentTarget.style.opacity = '1';
          }}
          onMouseOut={(e) => {
            e.currentTarget.style.opacity = '0.7';
          }}
          onFocus={(e) => {
            e.currentTarget.style.opacity = '1';
          }}
          onBlur={(e) => {
            e.currentTarget.style.opacity = '0.75';
          }}
        >
          ×
        </button>
      </div>

      {/* Messages */}
      <main
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '18px',
          display: 'flex',
          flexDirection: 'column',
          backgroundColor: 'var(--background)',
        }}
      >
        {messages.length === 0 && (
          <div
            style={{
              textAlign: 'center',
              color: 'var(--faded)',
              fontStyle: 'italic',
              marginTop: '20px',
            }}
          >
            👋 Hi! I'm here to help you with the documentation. Ask me anything!
          </div>
        )}

        {messages.map((message) => (
          <div key={message.id} style={{ alignSelf: message.isUser ? 'flex-end' : 'flex-start', maxWidth: '75%' }}>
            <div
              className={
                message.isUser ? 'baml-bubble baml-me' : 'baml-bubble baml-ai'
              }
              style={{
                padding: '10px 14px',
                borderRadius: '14px',
                fontSize: '14px',
                lineHeight: '1.5',
                marginBottom: message.ranked_docs && message.ranked_docs.length > 0 ? '8px' : '6px',
                boxShadow: '0 2px 6px rgba(0,0,0,.06)',
                backgroundColor: message.isUser ? 'var(--accent-primary)' : message.error ? '#fef2f2' : 'var(--card-background)',
                color: message.isUser ? '#fff' : message.error ? '#dc2626' : 'var(--text)',
                wordWrap: 'break-word',
                borderLeft: message.error ? '3px solid #dc2626' : undefined,
                border: message.isUser ? 'none' : '1px solid var(--border)',
              }}
            >
              {message.text}
              {message.error && (
                <button
                  onClick={() => handleRetry(message)}
                  style={{
                    marginLeft: '8px',
                    padding: '4px 8px',
                    fontSize: '12px',
                    backgroundColor: '#dc2626',
                    color: '#fff',
                    border: 'none',
                    borderRadius: '6px',
                    cursor: 'pointer',
                    fontWeight: '500',
                    transition: 'background-color 0.2s ease',
                  }}
                  onMouseOver={(e) => {
                    e.currentTarget.style.backgroundColor = '#b91c1c';
                  }}
                  onMouseOut={(e) => {
                    e.currentTarget.style.backgroundColor = '#dc2626';
                  }}
                >
                  Retry
                </button>
              )}
            </div>
            {message.ranked_docs && message.ranked_docs.length > 0 && (
              <div
                style={{
                  fontSize: '12px',
                  color: 'var(--faded)',
                  marginBottom: '6px',
                }}
              >
                <div style={{ fontWeight: '600', marginBottom: '4px' }}>
                  Related docs:
                </div>
                {message.ranked_docs.map((doc, index) => (
                  <div key={index} style={{ marginBottom: '2px' }}>
                    <a
                      href={doc.url}
                      style={{
                        color: 'var(--accent-primary)',
                        textDecoration: 'none',
                        fontSize: '12px',
                        transition: 'text-decoration 0.2s ease',
                      }}
                      onMouseOver={(e) => {
                        e.currentTarget.style.textDecoration = 'underline';
                      }}
                      onMouseOut={(e) => {
                        e.currentTarget.style.textDecoration = 'none';
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
            className="baml-bubble baml-ai"
            style={{
              maxWidth: '75%',
              padding: '10px 14px',
              borderRadius: '14px',
              fontSize: '14px',
              lineHeight: '1.5',
              marginBottom: '6px',
              boxShadow: '0 2px 6px rgba(0,0,0,.06)',
              alignSelf: 'flex-start',
              backgroundColor: 'var(--card-background)',
              color: 'var(--text)',
              border: '1px solid var(--border)',
              animation: 'pulse 1.5s ease-in-out infinite',
            }}
          >
            …thinking
          </div>
        )}

        <div ref={messagesEndRef} />
      </main>

      {/* Input Form */}
      <form
        onSubmit={handleSubmit}
        style={{
          display: 'flex',
          borderTop: '1px solid var(--border)',
          backgroundColor: 'var(--background)',
        }}
      >
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Type a question…"
          disabled={isLoading}
          style={{
            flex: 1,
            padding: '14px',
            border: 'none',
            fontSize: '14px',
            outline: 'none',
            fontFamily: 'inherit',
            backgroundColor: 'var(--background)',
            color: 'var(--text)',
          }}
        />
        <button
          type="submit"
          disabled={isLoading || !input.trim()}
          style={{
            border: 'none',
            padding: '0 20px',
            background: 'var(--accent-primary)',
            color: '#fff',
            fontWeight: '600',
            cursor: isLoading || !input.trim() ? 'not-allowed' : 'pointer',
            opacity: isLoading || !input.trim() ? 0.6 : 1,
            transition: 'opacity 0.2s ease',
          }}
        >
          Send
        </button>
      </form>

      <style>
        {`
          @keyframes pulse {
            0%, 100% { opacity: 0.7; }
            50% { opacity: 1; }
          }
          
          /* Dark mode support for chatbot background */
          .dark .chatbot-gradient {
            background: linear-gradient(
              180deg,
              rgba(183, 148, 255, 0.15) 0%,
              rgba(183, 148, 255, 0.05) 20%,
              rgba(0, 0, 0, 0) 40%
            ) !important;
          }
        `}
      </style>
    </div>
  );
};

export default ChatBot;
