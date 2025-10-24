// Singleton pattern to persist across hot reloads in development
// Using globalThis to store the queue

type PendingRequest = {
  resolve: (message: string) => void;
  timestamp: number;
  agentMessage?: string;
};

declare global {
  var __messageQueue: PendingRequest[] | undefined;
  var __pendingAgentMessage: string | null | undefined;
}

function getQueue(): PendingRequest[] {
  if (!globalThis.__messageQueue) {
    globalThis.__messageQueue = [];
  }
  return globalThis.__messageQueue;
}

export function addPendingRequest(request: PendingRequest): void {
  const queue = getQueue();
  queue.push(request);
  console.log(`[Queue] Added request. Total: ${queue.length}`);
}

export function removePendingRequest(timestamp: number): boolean {
  const queue = getQueue();
  const index = queue.findIndex((req) => req.timestamp === timestamp);
  if (index !== -1) {
    queue.splice(index, 1);
    console.log(`[Queue] Removed request. Total: ${queue.length}`);
    return true;
  }
  return false;
}

export function getPendingRequestCount(): number {
  return getQueue().length;
}

export function hasPendingRequests(): boolean {
  return getQueue().length > 0;
}

export function resolveAllRequests(message: string): number {
  const queue = getQueue();
  let count = 0;
  console.log(
    `[Queue] Resolving all requests with: "${message}". Queue length: ${queue.length}`,
  );

  while (queue.length > 0) {
    const request = queue.shift();
    if (request) {
      console.log(`[Queue] Resolving request ${count + 1}`);
      request.resolve(message);
      count++;
    }
  }

  console.log(`[Queue] Resolved ${count} requests. Remaining: ${queue.length}`);
  globalThis.__pendingAgentMessage = null; // Clear after resolving
  return count;
}

export function setPendingAgentMessage(message: string): void {
  globalThis.__pendingAgentMessage = message;
  console.log(`[Queue] Set pending agent message: "${message}"`);
}

export function getPendingAgentMessage(): string | null {
  return globalThis.__pendingAgentMessage || null;
}

export function clearPendingAgentMessage(): void {
  globalThis.__pendingAgentMessage = null;
}
