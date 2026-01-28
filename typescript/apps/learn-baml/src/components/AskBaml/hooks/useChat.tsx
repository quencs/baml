import { useState, useCallback, useEffect, useRef, createContext, useContext, ReactNode } from 'react';
import {
  initAnalytics,
  trackAssistantOpened,
  trackQuery,
  trackResponse,
  trackError,
  trackSessionEnded,
} from '../../../lib/analytics';

interface Message {
  id: string;
  role: 'user' | 'assistant';
  text: string;
  citations?: Array<{ title: string; url: string; relevance: string }>;
  suggested_questions?: string[];
}

interface ChatState {
  messages: Message[];
  isLoading: boolean;
  isOpen: boolean;
}

interface ChatContextValue extends ChatState {
  sendMessage: (text: string) => Promise<void>;
  setIsOpen: (isOpen: boolean, source?: 'button' | 'keyboard') => void;
  clearMessages: () => void;
  sessionId: string;
}

const ChatContext = createContext<ChatContextValue | null>(null);

export function useChat(): ChatContextValue {
  const context = useContext(ChatContext);
  if (!context) {
    throw new Error('useChat must be used within a ChatProvider');
  }
  return context;
}

interface ChatProviderProps {
  children: ReactNode;
}

export function ChatProvider({ children }: ChatProviderProps) {
  const [state, setState] = useState<ChatState>({
    messages: [],
    isLoading: false,
    isOpen: false,
  });

  const [sessionId] = useState(() => `session-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`);
  const sessionStartTime = useRef<number>(Date.now());

  // Initialize analytics
  useEffect(() => {
    initAnalytics();
  }, []);

  // Persist messages to localStorage
  useEffect(() => {
    if (typeof window === 'undefined') return;
    const saved = localStorage.getItem('ask-baml-messages');
    if (saved) {
      try {
        setState(prev => ({ ...prev, messages: JSON.parse(saved) }));
      } catch {
        // Ignore parse errors
      }
    }
  }, []);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    localStorage.setItem('ask-baml-messages', JSON.stringify(state.messages));
  }, [state.messages]);

  // Keyboard shortcuts
  useEffect(() => {
    if (typeof window === 'undefined') return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setState(prev => ({ ...prev, isOpen: !prev.isOpen }));
      }
      if (e.key === 'Escape' && state.isOpen) {
        setState(prev => ({ ...prev, isOpen: false }));
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [state.isOpen]);

  const setIsOpen = useCallback((isOpen: boolean, source?: 'button' | 'keyboard') => {
    if (isOpen && !state.isOpen) {
      sessionStartTime.current = Date.now();
      if (source) {
        trackAssistantOpened(source);
      }
    }

    // Track session end when closing
    if (!isOpen && state.isOpen && state.messages.length > 0) {
      trackSessionEnded({
        sessionId,
        messageCount: state.messages.length,
        durationSeconds: Math.round((Date.now() - sessionStartTime.current) / 1000),
      });
    }

    setState(prev => ({ ...prev, isOpen }));
  }, [state.isOpen, state.messages.length, sessionId]);

  const sendMessage = useCallback(async (text: string) => {
    if (!text.trim() || state.isLoading) return;

    const startTime = Date.now();
    const trimmedText = text.trim();

    // Track query
    trackQuery({
      query: trimmedText,
      sessionId,
      conversationLength: state.messages.length,
    });

    const userMessage: Message = {
      id: `user-${Date.now()}`,
      role: 'user',
      text: trimmedText,
    };

    setState(prev => ({
      ...prev,
      messages: [...prev.messages, userMessage],
      isLoading: true,
    }));

    try {
      const response = await fetch('/api/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          message: trimmedText,
          prev_messages: state.messages.map(m => ({ role: m.role, text: m.text })),
        }),
      });

      if (!response.ok) throw new Error('Request failed');

      const data = await response.json();

      // Track successful response
      trackResponse({
        query: trimmedText,
        answer: data.answer,
        citations: data.citations || [],
        suggestedQuestions: data.suggested_questions || [],
        latencyMs: Date.now() - startTime,
        topDocScores: data._debug?.doc_scores || [],
      });

      const assistantMessage: Message = {
        id: `assistant-${Date.now()}`,
        role: 'assistant',
        text: data.answer,
        citations: data.citations,
        suggested_questions: data.suggested_questions,
      };

      setState(prev => ({
        ...prev,
        messages: [...prev.messages, assistantMessage],
        isLoading: false,
      }));
    } catch (error) {
      console.error('Chat error:', error);

      // Track error
      trackError({
        query: trimmedText,
        errorType: error instanceof Error ? error.name : 'UnknownError',
        errorMessage: error instanceof Error ? error.message : 'Unknown error',
      });

      setState(prev => ({
        ...prev,
        messages: [
          ...prev.messages,
          {
            id: `error-${Date.now()}`,
            role: 'assistant',
            text: 'Sorry, something went wrong. Please try again.',
          },
        ],
        isLoading: false,
      }));
    }
  }, [state.messages, state.isLoading, sessionId]);

  const clearMessages = useCallback(() => {
    setState(prev => ({ ...prev, messages: [] }));
  }, []);

  const value: ChatContextValue = {
    ...state,
    sendMessage,
    setIsOpen,
    clearMessages,
    sessionId,
  };

  return (
    <ChatContext.Provider value={value}>
      {children}
    </ChatContext.Provider>
  );
}
