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
// ************* APIs *******************
// **************************************
class APIMessage {
  message string
}

function TalkToUser(message: string) -> string {
  let resp = baml.fetch_as<APIMessage>(baml.HttpRequest {
    url: "http://localhost:3001/api/user_message",
    method: baml.HttpMethod.Post,
    json: APIMessage { message: message }
  });
  resp.message

}


// **************************************
// *********** CONTEXT ******************
// **************************************

function LLMChooseTool(history: Message[], user_ctx: TravelAgentContext) -> (RequestInfoFromUser | UpdateTravelAgentContext | SearchFlights)[] {
  client CustomSonnet4
  prompt #"
    {{ ctx.output_format }}
    You are a helpful travel agent. Here is what you already know about
    the customer:
    {{ user_ctx }}

    Issue commands in the following format to interact with the user and
    to build an itinerary around their interests, preferred destination
    and time constraints.

    {{ ctx.output_format }}

    If the conversation history is empty, start with a greeting.

    Here is the full conversation history:
    {{ history }}
  "#
}

function Main() -> int {
  watch let debug = "";
  debug = "Hello";
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
		if (t instanceof RequestInfoFromUser) {
		  current_tool_type = t.type;
		  debug = "Got RequestInfoFromUser";
		  messages.push(Message { sender: "agent", content: t.message_to_user });
		  debug = "Pushed message";
		  let userResponse = TalkToUser(t.message_to_user);
		  debug = "Got Response! " + userResponse;
		  messages.push(Message { sender: "user", content: userResponse} );
		  debug = "Pushed response";
		} else {
		  debug = "FALLTHROUGH!!!";
		  break;
		}
	 }
  }
  debug = "finished for somehow";

  0
}
