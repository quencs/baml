import React, { useRef, useEffect, useState, useCallback } from 'react';
import ReactMarkdown from 'react-markdown';
import { X, Trash2, Send, GripVertical } from 'lucide-react';
import { useChat } from './hooks/useChat';
import { trackCitationClick, trackSuggestionClick } from '../../lib/analytics';
import { Feedback } from './Feedback';
import styles from './AskBamlSidePanel.module.css';

const MIN_PANEL_WIDTH = 280;
const MAX_PANEL_WIDTH = 600;
const DEFAULT_PANEL_WIDTH = 380;

export function AskBamlSidePanel() {
  const { messages, isLoading, isOpen, setIsOpen, sendMessage, clearMessages } = useChat();
  const [input, setInput] = useState('');
  const [panelWidth, setPanelWidth] = useState(DEFAULT_PANEL_WIDTH);
  const [isResizing, setIsResizing] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  // Scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Focus input when panel opens
  useEffect(() => {
    if (isOpen) {
      setTimeout(() => inputRef.current?.focus(), 100);
    }
  }, [isOpen]);

  // Handle resize
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newWidth = window.innerWidth - e.clientX;
      setPanelWidth(Math.min(MAX_PANEL_WIDTH, Math.max(MIN_PANEL_WIDTH, newWidth)));
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizing]);

  // Add body class when panel is open to adjust main content
  useEffect(() => {
    if (isOpen) {
      document.documentElement.style.setProperty('--ask-panel-width', `${panelWidth}px`);
      document.body.classList.add('ask-panel-open');
    } else {
      document.body.classList.remove('ask-panel-open');
    }

    return () => {
      document.body.classList.remove('ask-panel-open');
    };
  }, [isOpen, panelWidth]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (input.trim()) {
      sendMessage(input);
      setInput('');
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
  };

  // Get the last user message for tracking
  const lastUserMessage = messages.filter(m => m.role === 'user').pop();
  const lastMessage = messages[messages.length - 1];

  const handleCitationClick = (cite: { title: string; url: string }) => {
    if (lastUserMessage) {
      trackCitationClick({
        citationUrl: cite.url,
        citationTitle: cite.title,
        query: lastUserMessage.text,
      });
    }
  };

  const handleSuggestionClick = (suggestion: string) => {
    if (lastUserMessage) {
      trackSuggestionClick({
        suggestion,
        originalQuery: lastUserMessage.text,
      });
    }
    sendMessage(suggestion);
  };

  if (!isOpen) return null;

  return (
    <div
      ref={panelRef}
      className={styles.panel}
      style={{ width: panelWidth }}
      data-resizing={isResizing}
    >
      {/* Resize Handle */}
      <div
        className={styles.resizeHandle}
        onMouseDown={handleMouseDown}
      >
        <GripVertical size={12} className={styles.resizeGrip} />
      </div>

      <div className={styles.chatContainer}>
        {/* Header */}
        <header className={styles.header}>
          <span className={styles.title}>Ask AI</span>
          <div className={styles.headerActions}>
            {messages.length > 0 && (
              <button
                onClick={clearMessages}
                className={styles.iconButton}
                title="Clear conversation"
              >
                <Trash2 size={14} />
              </button>
            )}
            <button
              onClick={() => setIsOpen(false)}
              className={styles.iconButton}
              title="Close panel"
            >
              <X size={16} />
            </button>
          </div>
        </header>

        {/* Messages */}
        <main className={styles.messages}>
          {messages.length === 0 ? (
            <div className={styles.welcome}>
              <div className={styles.welcomeIcon}>?</div>
              <h3>Ask about BAML</h3>
              <p>I can help you with syntax, concepts, examples, and more.</p>
              <div className={styles.suggestions}>
                {[
                  'How do I define a function?',
                  'What are clients in BAML?',
                  'How does type validation work?',
                ].map((q, i) => (
                  <button
                    key={i}
                    onClick={() => sendMessage(q)}
                    className={styles.suggestionChip}
                  >
                    {q}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            messages.map(msg => (
              <div
                key={msg.id}
                className={`${styles.message} ${styles[msg.role]}`}
              >
                {msg.role === 'assistant' ? (
                  <ReactMarkdown>{msg.text}</ReactMarkdown>
                ) : (
                  msg.text
                )}
              </div>
            ))
          )}

          {isLoading && (
            <div className={`${styles.message} ${styles.assistant}`}>
              <div className={styles.loadingDots}>
                <span />
                <span />
                <span />
              </div>
            </div>
          )}

          <div ref={messagesEndRef} />
        </main>

        {/* Citations */}
        {lastMessage?.citations && lastMessage.citations.length > 0 && (
          <div className={styles.citations}>
            <span className={styles.citationsLabel}>Sources</span>
            <div className={styles.citationsList}>
              {lastMessage.citations.map((cite, i) => (
                <a
                  key={i}
                  href={cite.url}
                  className={styles.citation}
                  onClick={() => handleCitationClick(cite)}
                >
                  {cite.title}
                </a>
              ))}
            </div>
          </div>
        )}

        {/* Suggested follow-ups */}
        {lastMessage?.suggested_questions && lastMessage.suggested_questions.length > 0 && (
          <div className={styles.followUps}>
            {lastMessage.suggested_questions.map((q, i) => (
              <button
                key={i}
                onClick={() => handleSuggestionClick(q)}
                className={styles.followUpChip}
              >
                {q}
              </button>
            ))}
          </div>
        )}

        {/* Feedback */}
        {lastMessage?.role === 'assistant' && lastUserMessage && (
          <Feedback
            query={lastUserMessage.text}
            answer={lastMessage.text}
          />
        )}

        {/* Input */}
        <form onSubmit={handleSubmit} className={styles.inputForm}>
          <textarea
            ref={inputRef}
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Ask a question..."
            disabled={isLoading}
            rows={1}
            className={styles.input}
          />
          <button
            type="submit"
            disabled={isLoading || !input.trim()}
            className={styles.sendButton}
          >
            <Send size={16} />
          </button>
        </form>
      </div>
    </div>
  );
}
