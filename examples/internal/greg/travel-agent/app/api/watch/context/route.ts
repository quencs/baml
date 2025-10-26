import { NextRequest, NextResponse } from "next/server";

export async function POST(request: NextRequest) {
  try {
    const context = await request.json();
    console.log("[API /watch/context] Received context update:", context);

    // Store in a global variable that can be polled
    // In a real app, you'd use a proper state management solution
    global.travelAgentContext = context;

    return NextResponse.json({ success: true });
  } catch (error) {
    console.error("[API /watch/context] Error:", error);
    return NextResponse.json(
      { error: "Failed to update context" },
      { status: 500 }
    );
  }
}

export async function GET() {
  const context = global.travelAgentContext || {
    nAdults: null,
    nChildren: null,
    interests: [],
    homeLocation: null,
    dateRange: null,
  };

  return NextResponse.json(context);
}
