import { NextResponse } from "next/server";
import { resolveAllRequests } from "@/app/lib/messageQueue";

export async function POST(request: Request) {
  const { message } = await request.json();

  console.log(`[Resolve] Received message: "${message}"`);

  if (!message) {
    return NextResponse.json({ error: "Message is required" }, { status: 400 });
  }

  // Resolve all pending requests with this message
  const resolvedCount = resolveAllRequests(message);

  console.log(`[Resolve] Resolved ${resolvedCount} requests`);
  return NextResponse.json({ success: true, resolvedCount });
}
