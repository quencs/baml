import { NextResponse } from "next/server";
import {
  getPendingRequestCount,
  getPendingAgentMessage,
} from "@/app/lib/messageQueue";

export async function GET() {
  const count = getPendingRequestCount();
  const pending = count > 0;
  const agentMessage = getPendingAgentMessage();
  console.log(
    `[Status] Pending requests: ${count}, agent message: ${agentMessage ? `"${agentMessage}"` : "none"}`,
  );
  return NextResponse.json({ pending, count, agentMessage });
}
