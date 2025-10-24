"use client";

import { useEffect, useRef, useState } from "react";
import { MessageBubble, type Message } from "./MessageBubble";
import { ActiveToolCallBanner } from "./ActiveToolCallBanner";

interface ChatPanelProps {
  messages: Message[];
  activeToolCall?: string;
  onSendMessage?: (message: string) => void;
  shouldFlash?: boolean;
}

export function ChatPanel({
  messages,
  activeToolCall,
  onSendMessage,
  shouldFlash = false,
}: ChatPanelProps) {
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [isFlashing, setIsFlashing] = useState(false);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Handle focus and flash when shouldFlash changes
  useEffect(() => {
    if (shouldFlash) {
      inputRef.current?.focus();
      setIsFlashing(true);

      // Stop flashing after animation completes
      const timer = setTimeout(() => {
        setIsFlashing(false);
      }, 2000);

      return () => clearTimeout(timer);
    }
  }, [shouldFlash]);

  const handleSend = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const message = formData.get("message") as string;
    if (message.trim() && onSendMessage) {
      onSendMessage(message);
      e.currentTarget.reset();
    }
  };

  return (
    <div className="flex-1 flex flex-col bg-white rounded-2xl shadow-xl overflow-hidden">
      {/* Chat Header */}
      <div className="bg-gradient-to-r from-blue-600 to-purple-600 p-4">
        <h2 className="text-xl font-semibold text-white">
          Chat & Agent Status
        </h2>
      </div>

      {/* Conversation History */}
      <div className="flex-1 overflow-y-auto p-6 space-y-4">
        {messages.length === 0 ? (
          <div className="text-gray-500 text-center py-8">
            <div className="text-6xl mb-4">💬</div>
            <p>Start a conversation to plan your trip</p>
          </div>
        ) : (
          <>
            {messages.map((message) => (
              <MessageBubble key={message.id} message={message} />
            ))}
            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      {/* Active Tool Call Banner */}
      <ActiveToolCallBanner
        toolName={activeToolCall || ""}
        isActive={!!activeToolCall}
      />

      {/* User Input */}
      <div className="p-4 bg-gray-50 border-t border-gray-200">
        <form onSubmit={handleSend} className="flex gap-3">
          <input
            ref={inputRef}
            type="text"
            name="message"
            placeholder="Type your message..."
            className={`flex-1 px-4 py-3 bg-white border border-gray-300 rounded-xl focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition-all ${
              isFlashing ? "animate-flash ring-4 ring-amber-400" : ""
            }`}
          />
          <button
            type="submit"
            className="px-6 py-3 bg-gradient-to-r from-blue-600 to-purple-600 text-white rounded-xl font-medium hover:shadow-lg transition-all duration-200 hover:scale-105"
          >
            Send
          </button>
        </form>
      </div>
    </div>
  );
}
