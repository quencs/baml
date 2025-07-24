import React, { useState, useRef, useEffect } from 'react';

interface Message {
  id: string;
  text: string;
  isUser: boolean;
  timestamp: Date;
}

interface ChatBotProps {
  apiEndpoint?: string;
  isOpen?: boolean;
  onClose?: () => void;
}

const ChatBot: React.FC<ChatBotProps> = ({ 
  apiEndpoint = 'http://localhost:3002/api/docs-chat',
  isOpen = false,
  onClose
}) => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const sendMessage = async (text: string) => {
    if (!text.trim()) return;

    const userMessage: Message = {
      id: Date.now().toString(),
      text: text.trim(),
      isUser: true,
      timestamp: new Date(),
    };

    setMessages(prev => [...prev, userMessage]);
    setInput('');
    setIsLoading(true);

    try {
      const response = await fetch(apiEndpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ message: text.trim() }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();
      
      const botMessage: Message = {
        id: (Date.now() + 1).toString(),
        text: data.response || 'Sorry, I couldn\'t process your request.',
        isUser: false,
        timestamp: new Date(),
      };

      setMessages(prev => [...prev, botMessage]);
    } catch (error) {
      console.error('Error sending message:', error);
      const errorMessage: Message = {
        id: (Date.now() + 1).toString(),
        text: 'Sorry, there was an error processing your request. Please try again.',
        isUser: false,
        timestamp: new Date(),
      };
      setMessages(prev => [...prev, errorMessage]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    sendMessage(input);
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
      const header = document.querySelector('header, .fern-header') as HTMLElement;
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
    <div style={{
      position: 'fixed',
      top: `${panelTop}px`,
      right: '0',
      width: '380px',
      height: panelHeight,
      backgroundColor: 'white',
      borderLeft: '1px solid #e2e8f0',
      boxShadow: '-4px 0 32px rgba(0,0,0,0.08)',
      transform: isOpen ? 'translateX(0)' : 'translateX(100%)',
      transition: 'transform .3s cubic-bezier(.4,0,.2,1)',
      display: 'flex',
      flexDirection: 'column',
      zIndex: 2000,
      fontFamily: 'Inter, system-ui, -apple-system, BlinkMacSystemFont, sans-serif',
    }}>
      {/* Header */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        height: '56px',
        padding: '0 20px',
        fontSize: '15px',
        fontWeight: '600',
        background: '#7c3aed',
        color: '#fff'
      }}>
        <span>BAML AI</span>
        <button
          onClick={handleClose}
          style={{
            background: 'none',
            border: 'none',
            fontSize: '26px',
            color: '#fff',
            cursor: 'pointer',
            opacity: 0.75,
            padding: '0',
            lineHeight: 1
          }}
          onMouseOver={(e) => {
            e.currentTarget.style.opacity = '1';
          }}
          onMouseOut={(e) => {
            e.currentTarget.style.opacity = '0.75';
          }}
        >
          ×
        </button>
      </div>

      {/* Messages */}
      <main style={{
        flex: 1,
        overflowY: 'auto',
        padding: '18px',
        display: 'flex',
        flexDirection: 'column'
      }}>
        {messages.length === 0 && (
          <div style={{
            textAlign: 'center',
            color: '#666',
            fontStyle: 'italic',
            marginTop: '20px',
          }}>
            👋 Hi! I'm here to help you with the documentation. Ask me anything!
          </div>
        )}
        
        {messages.map((message) => (
          <div
            key={message.id}
            className={message.isUser ? 'baml-bubble baml-me' : 'baml-bubble baml-ai'}
            style={{
              maxWidth: '75%',
              padding: '10px 14px',
              borderRadius: '14px',
              fontSize: '14px',
              lineHeight: '1.5',
              marginBottom: '6px',
              boxShadow: '0 2px 6px rgba(0,0,0,.06)',
              alignSelf: message.isUser ? 'flex-end' : 'flex-start',
              backgroundColor: message.isUser ? '#7c3aed' : '#f3f4f6',
              color: message.isUser ? '#fff' : '#111827',
              wordWrap: 'break-word',
            }}
          >
            {message.text}
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
              backgroundColor: '#f3f4f6',
              color: '#111827',
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
          borderTop: '1px solid #e5e7eb'
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
            fontFamily: 'inherit'
          }}
        />
        <button
          type="submit"
          disabled={isLoading || !input.trim()}
          style={{
            border: 'none',
            padding: '0 20px',
            background: '#7c3aed',
            color: '#fff',
            fontWeight: '600',
            cursor: isLoading || !input.trim() ? 'not-allowed' : 'pointer',
            opacity: isLoading || !input.trim() ? 0.6 : 1,
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
        `}
      </style>
    </div>
  );
};

export default ChatBot;