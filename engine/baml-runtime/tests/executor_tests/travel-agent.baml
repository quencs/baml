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


class TravelAgentContext {
  nAdults int?
  nChildren int?
  interests string[]
  homeLocation string?
  dateRange string?

}

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


function LLMChooseTool(history: Message[], user_ctx: TravelAgentContext) -> (RequestInfoFromUser | UpdateTravelAgentContext | SearchFlights)[] {
  [RequestInfoFromUser{type: "request_info_from_user", message_to_user: "Hey wassup" }]
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
		  // messages.push(Message { sender: "agent", content: t.message_to_user });
		  // let userResponse = TalkToUser(t.message_to_user);
		  // messages.push(Message { sender: "user", content: userResponse} );
		} else {
		  debug = "FALLTHROUGH!!!";
		}
	 }
  }
  debug = "finished for somehow";

  0
}

//> Main()
//
// 10
