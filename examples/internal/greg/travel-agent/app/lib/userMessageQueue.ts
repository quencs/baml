// Global queue for pending user message requests
// This needs to be in a shared module so both the API route and client can access it

let messageResolver: ((message: string) => void) | null = null;

export function setMessageResolver(resolver: (message: string) => void) {
  messageResolver = resolver;
}

export function resolveMessage(message: string) {
  if (messageResolver) {
    messageResolver(message);
    messageResolver = null;
  }
}

export function hasResolver(): boolean {
  return messageResolver !== null;
}
