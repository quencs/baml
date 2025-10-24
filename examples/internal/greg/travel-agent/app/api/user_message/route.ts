import { NextResponse } from "next/server";
import {
  addPendingRequest,
  setPendingAgentMessage,
} from "@/app/lib/messageQueue";

export async function POST(request: Request) {
  console.log("[POST /user_message] New request received");

  // Parse the agent message from the request body
  let agentMessage: string | undefined;
  try {
    const body = await request.json();
    agentMessage = body.message;
    console.log(`[POST /user_message] Agent message: "${agentMessage}"`);
  } catch (error) {
    console.log("[POST /user_message] No agent message in body");
  }

  // Store the agent message so the client can retrieve it
  if (agentMessage) {
    setPendingAgentMessage(agentMessage);
  }

  return new Promise<NextResponse>((resolve) => {
    const timestamp = Date.now();

    // Add this request to the queue (no timeout - waits indefinitely)
    addPendingRequest({
      resolve: (message: string) => {
        console.log(
          `[POST /user_message] Resolving with user message: "${message}"`,
        );
        resolve(NextResponse.json({ message }));
      },
      timestamp,
      agentMessage,
    });

    console.log(
      "[POST /user_message] Request added to queue, waiting for user input (no timeout)",
    );
  });
}

export async function GET() {
  console.log("[GET /user_message] New request received");
  return new Promise<NextResponse>((resolve) => {
    const timestamp = Date.now();

    // Add this request to the queue (no timeout - waits indefinitely)
    addPendingRequest({
      resolve: (message: string) => {
        console.log(`[GET /user_message] Resolving with message: "${message}"`);
        resolve(NextResponse.json({ message }));
      },
      timestamp,
    });

    console.log(
      "[GET /user_message] Request added to queue, waiting for user input (no timeout)",
    );
  });
}
