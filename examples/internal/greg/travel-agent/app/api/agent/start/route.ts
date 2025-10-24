import { NextResponse } from "next/server";
import { startAgent } from "@/app/lib/agentRunner";

export async function POST() {
  console.log("[API] Received request to start agent");

  try {
    startAgent();
    return NextResponse.json({ success: true, message: "Agent started" });
  } catch (error) {
    console.error("[API] Failed to start agent:", error);
    return NextResponse.json(
      { success: false, error: "Failed to start agent" },
      { status: 500 }
    );
  }
}
