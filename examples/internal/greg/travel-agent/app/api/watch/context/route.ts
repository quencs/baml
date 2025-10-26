import { NextRequest, NextResponse } from "next/server";

// Extend global type
declare global {
  var travelAgentContext: any;
}

// Initialize if not exists
if (!global.travelAgentContext) {
  global.travelAgentContext = {
    nAdults: null,
    nChildren: null,
    interests: [],
    homeLocation: null,
    dateRange: null,
  };
}

export async function POST(request: NextRequest) {
  try {
    const watchEvent = await request.json();
    console.log(
      "[API /watch/context POST] Received context update:",
      JSON.stringify(watchEvent, null, 2),
    );

    // Extract the actual context value from the watch event
    const context = watchEvent.value || watchEvent;

    // Store in a global variable that can be polled
    global.travelAgentContext = context;

    console.log(
      "[API /watch/context POST] Stored in global:",
      JSON.stringify(global.travelAgentContext, null, 2),
    );

    return NextResponse.json({ success: true });
  } catch (error) {
    console.error("[API /watch/context POST] Error:", error);
    return NextResponse.json(
      { error: "Failed to update context" },
      { status: 500 },
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
