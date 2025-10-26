"use client";

import { useEffect, useRef, useState } from "react";
import { useAtom, useAtomValue, useSetAtom } from "jotai";
import confetti from "canvas-confetti";
import { ChatPanel } from "./components/ChatPanel";
import { TravelPlanPanel } from "./components/TravelPlanPanel";
import { ContextPanel } from "./components/ContextPanel";
import {
  messagesAtom,
  activeToolAtom,
  itineraryAtom,
  addMessageAtom,
  pendingUserInputAtom,
  travelAgentContextAtom,
  bamlItineraryAtom,
} from "./store/atoms";

export default function Home() {
  const messages = useAtomValue(messagesAtom);
  const activeTool = useAtomValue(activeToolAtom);
  const itinerary = useAtomValue(itineraryAtom);
  const context = useAtomValue(travelAgentContextAtom);
  const bamlItinerary = useAtomValue(bamlItineraryAtom);
  const addMessage = useSetAtom(addMessageAtom);
  const [pendingUserInput, setPendingUserInput] = useAtom(pendingUserInputAtom);
  const [, setContext] = useAtom(travelAgentContextAtom);
  const [, setBAMLItinerary] = useAtom(bamlItineraryAtom);
  const [shouldFlash, setShouldFlash] = useState(false);
  const [hasItinerary, setHasItinerary] = useState(false);
  const resolverRef = useRef<((message: string) => void) | null>(null);
  const pendingUserInputRef = useRef(pendingUserInput);

  // Keep ref in sync with state to avoid stale closures
  pendingUserInputRef.current = pendingUserInput;

  // Start the agent when the page loads
  useEffect(() => {
    console.log("[Page] Starting agent on mount...");
    fetch("/api/agent/start", { method: "POST" })
      .then((res) => res.json())
      .then((data) => console.log("[Page] Agent start response:", data))
      .catch((err) => console.error("[Page] Failed to start agent:", err));
  }, []); // Run once on mount

  // Poll for pending API requests
  useEffect(() => {
    let isMounted = true;

    const poll = async () => {
      if (!isMounted) return;

      try {
        const response = await fetch("/api/user_message/status");
        if (response.ok && isMounted) {
          const data = await response.json();
          console.log(
            "Poll result:",
            data,
            "Current pending:",
            pendingUserInputRef.current,
          );

          if (data.pending && !pendingUserInputRef.current) {
            console.log("Triggering flash!");

            // Add agent message to chat history if present
            if (data.agentMessage) {
              console.log("Adding agent message to chat:", data.agentMessage);
              addMessage({
                content: data.agentMessage,
                timestamp: new Date().toLocaleTimeString("en-US", {
                  hour: "numeric",
                  minute: "2-digit",
                }),
                isAgent: true,
              });
            }

            setPendingUserInput(true);
            setShouldFlash(true);
            setTimeout(() => {
              if (isMounted) setShouldFlash(false);
            }, 2000);
          } else if (!data.pending && pendingUserInputRef.current) {
            setPendingUserInput(false);
          }
        }
      } catch (error) {
        console.error("Polling error:", error);
      }
    };

    const pollInterval = setInterval(poll, 2000);
    poll(); // Initial poll

    return () => {
      isMounted = false;
      clearInterval(pollInterval);
    };
  }, [setPendingUserInput, addMessage]); // Added addMessage to deps

  // Poll for context updates
  useEffect(() => {
    let isMounted = true;

    const pollContext = async () => {
      if (!isMounted) return;

      try {
        const response = await fetch("/api/watch/context");
        if (response.ok && isMounted) {
          const contextData = await response.json();
          setContext(contextData);
        }
      } catch (error) {
        // Silently fail - polling errors are not critical
      }
    };

    const pollInterval = setInterval(pollContext, 2000); // Poll every 2s for context updates
    pollContext(); // Initial poll

    return () => {
      isMounted = false;
      clearInterval(pollInterval);
    };
  }, [setContext]);

  // Poll for itinerary updates
  useEffect(() => {
    let isMounted = true;

    const pollItinerary = async () => {
      if (!isMounted) return;

      try {
        const response = await fetch("/api/watch/itinerary");
        if (response.ok && isMounted) {
          const itineraryData = await response.json();
          setBAMLItinerary(itineraryData);
        }
      } catch (error) {
        // Silently fail - polling errors are not critical
      }
    };

    const pollInterval = setInterval(pollItinerary, 2000); // Poll every 2s for itinerary updates
    pollItinerary(); // Initial poll

    return () => {
      isMounted = false;
      clearInterval(pollInterval);
    };
  }, [setBAMLItinerary]);

  // Detect when itinerary gets set and trigger confetti
  useEffect(() => {
    const hasContent =
      bamlItinerary &&
      (bamlItinerary.flights.length > 0 || bamlItinerary.activities.length > 0);

    if (hasContent && !hasItinerary) {
      // Itinerary just got set - trigger confetti!
      setHasItinerary(true);

      // Fire confetti animation
      const duration = 3000;
      const animationEnd = Date.now() + duration;
      const defaults = { startVelocity: 30, spread: 360, ticks: 60, zIndex: 0 };

      function randomInRange(min: number, max: number) {
        return Math.random() * (max - min) + min;
      }

      const interval: NodeJS.Timeout = setInterval(function () {
        const timeLeft = animationEnd - Date.now();

        if (timeLeft <= 0) {
          return clearInterval(interval);
        }

        const particleCount = 50 * (timeLeft / duration);

        confetti({
          ...defaults,
          particleCount,
          origin: { x: randomInRange(0.1, 0.3), y: Math.random() - 0.2 },
        });
        confetti({
          ...defaults,
          particleCount,
          origin: { x: randomInRange(0.7, 0.9), y: Math.random() - 0.2 },
        });
      }, 250);
    } else if (!hasContent && hasItinerary) {
      // Itinerary was cleared
      setHasItinerary(false);
    }
  }, [bamlItinerary, hasItinerary]);

  // Register message resolver when pending input is active
  useEffect(() => {
    if (pendingUserInput) {
      const resolver = (message: string) => {
        fetch("/api/user_message/resolve", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ message }),
        }).catch((err) => console.error("Failed to resolve:", err));
        setPendingUserInput(false);
      };
      resolverRef.current = resolver;
    } else {
      resolverRef.current = null;
    }
  }, [pendingUserInput, setPendingUserInput]);

  const handleSendMessage = (message: string) => {
    // Add user message to chat history
    addMessage({
      content: message,
      timestamp: new Date().toLocaleTimeString("en-US", {
        hour: "numeric",
        minute: "2-digit",
      }),
      isAgent: false,
    });

    // If there's a pending API request, resolve it
    if (resolverRef.current) {
      resolverRef.current(message);
      resolverRef.current = null;
    }

    // TODO: Call your BAML agent here and add agent response
    // Example:
    // const response = await callBAMLAgent(message);
    // addMessage({
    //   content: response,
    //   timestamp: new Date().toLocaleTimeString(...),
    //   isAgent: true,
    // });
  };

  const handleExportItinerary = () => {
    console.log("Exporting itinerary", itinerary);
    // TODO: Implement export logic (download JSON, PDF, etc.)
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 via-purple-50 to-pink-50">
      <div className="container mx-auto p-6 h-screen flex flex-col">
        {/* Header */}
        <div className="mb-6">
          <h1 className="text-4xl font-bold bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
            ✈️ AI Travel Agent
          </h1>
          <p className="text-gray-600 mt-2">
            Plan your perfect journey with AI assistance
          </p>
        </div>

        {/* Main Content */}
        <div className="flex flex-row gap-6 flex-1 overflow-hidden">
          <ChatPanel
            messages={messages}
            activeToolCall={activeTool || undefined}
            onSendMessage={handleSendMessage}
            shouldFlash={shouldFlash}
          />
          <div className="w-96 flex flex-col gap-6 overflow-hidden">
            {!hasItinerary && (
              <div className="flex-1 min-h-0">
                <ContextPanel context={context} />
              </div>
            )}
            <div className={hasItinerary ? "h-full" : "flex-1 min-h-0"}>
              <TravelPlanPanel
                planItems={itinerary}
                bamlItinerary={bamlItinerary}
                onExport={handleExportItinerary}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
