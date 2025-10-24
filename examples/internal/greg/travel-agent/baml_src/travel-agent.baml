// **************************************
// ************ TOOLS *******************
// **************************************
class RequestInfoFromUser {
  type "request_info_from_user"
  message_to_user string
}

class UpdateTravelAgentContext {
  type "update_travel_agent_context"
  new_context TravelAgentContext
}

class SearchFlights {
  type "search_flights"
}

class SetItinerary {
  type "set_itinerary"
  itinerary Itinerary
}

type Tool = RequestInfoFromUser | UpdateTravelAgentContext | SearchFlights


// **************************************
// ************ TYPES *******************
// **************************************

class Message {
  sender "user" | "agent"
  content string
}

class Itinerary {
  flights Flight[]
  activities Activity[]
}

class Flight {
  source string
  dest string
  datetime string
}

class Activity {
  name string
  location string
  price_dollars int
}


// **************************************
// *********** CONTEXT ******************
// **************************************
class TravelAgentContext {
  nAdults int?
  nChildren int?
  interests string[]
  homeLocation string?
  dateRange string?
}

// **************************************
// *********** CONTEXT ******************
// **************************************

function LLMChooseTool(history: Message[], ctx: TravelAgentContext) -> Tool[] {
  client CustomSonnet4
  prompt #"
    You are a helpful travel agent. Here is what you already know about
    the customer:
    {{ ctx }}

    Issue commands in the following format to interact with the user and
    to build an itinerary around their interests, preferred destination
    and time constraints.

    {{ ctx.output_format }}

    Here is the full conversation history:
    {{ history }}
  "#
}

function Main() -> int {
  let messages: Message[] = [];
  let context = TravelAgentContext {
    nAdults: null,
    nChildren: null,
    interests: [],
    homeLocation: null,
    dateRange: null
  };
  watch let current_tool_type = "no tool yet";

  for (;;) {
	 let tools = LLMChooseTool(messages, context);
	 for (let t in tools) {
		current_tool_type = t.type;
		if (t instanceOf RequestInfoFromUser) {

		}
		// if (t instanceOf RequestInfoFromUser) {
		  // messages.push(Message { sender: "agent", content: t.message });
		  // let userResponse = TalkToUser(t.message);
		  // messages.push(Message { sender: "user", content: userResponse.message} );
		// }
	 }
  }

  0
}
