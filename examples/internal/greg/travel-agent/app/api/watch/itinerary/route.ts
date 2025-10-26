import { NextRequest, NextResponse } from "next/server";

// Extend global type
declare global {
  var travelAgentItinerary: any;
}

// Initialize if not exists
if (!global.travelAgentItinerary) {
  global.travelAgentItinerary = {
    flights: [],
    activities: [],
  };
}

export async function POST(request: NextRequest) {
  try {
    const watchEvent = await request.json();
    console.log(
      "[API /watch/itinerary POST] Received itinerary update:",
      JSON.stringify(watchEvent, null, 2),
    );

    // Extract the actual itinerary value from the watch event
    const itinerary = watchEvent.value || watchEvent;

    // Store in a global variable that can be polled
    global.travelAgentItinerary = itinerary;

    console.log(
      "[API /watch/itinerary POST] Stored in global:",
      JSON.stringify(global.travelAgentItinerary, null, 2),
    );

    return NextResponse.json({ success: true });
  } catch (error) {
    console.error("[API /watch/itinerary POST] Error:", error);
    return NextResponse.json(
      { error: "Failed to update itinerary" },
      { status: 500 },
    );
  }
}

export async function GET() {
  const itinerary = global.travelAgentItinerary || {
    flights: [],
    activities: [],
  };

  return NextResponse.json(itinerary);
}
