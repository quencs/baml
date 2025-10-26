import { atom } from "jotai";
import type { Message } from "../components/MessageBubble";
import type { PlanItem } from "../components/TravelPlanItem";

// Chat message history
export const messagesAtom = atom<Message[]>([]);

// Currently executing tool (null when no tool is active)
export const activeToolAtom = atom<string | null>(null);

// Current travel itinerary
export const itineraryAtom = atom<PlanItem[]>([]);

// Derived atom to add a new message
export const addMessageAtom = atom(
  null,
  (get, set, newMessage: Omit<Message, "id">) => {
    const messages = get(messagesAtom);
    const id = `msg-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    set(messagesAtom, [...messages, { ...newMessage, id }]);
  },
);

// Derived atom to add a new itinerary item
export const addItineraryItemAtom = atom(
  null,
  (get, set, newItem: Omit<PlanItem, "id">) => {
    const items = get(itineraryAtom);
    const id = `item-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    set(itineraryAtom, [...items, { ...newItem, id }]);
  },
);

// Derived atom to clear all messages
export const clearMessagesAtom = atom(null, (get, set) => {
  set(messagesAtom, []);
});

// Derived atom to clear itinerary
export const clearItineraryAtom = atom(null, (get, set) => {
  set(itineraryAtom, []);
});

// Atom to track if there's a pending user message request from the API
export const pendingUserInputAtom = atom<boolean>(false);

// TravelAgentContext from watch variable
export interface TravelAgentContext {
  nAdults: number | null;
  nChildren: number | null;
  interests: string[];
  homeLocation: string | null;
  dateRange: string | null;
}

export const travelAgentContextAtom = atom<TravelAgentContext>({
  nAdults: null,
  nChildren: null,
  interests: [],
  homeLocation: null,
  dateRange: null,
});
