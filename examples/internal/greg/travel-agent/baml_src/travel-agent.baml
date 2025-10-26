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
  departure_id string @description("like IAD or AUS")
  arrival_id string @description("like IAD or AUS")
  adults int
  children int
  outbound_date string @description("YYYY-MM-DD")
  return_date string @description("YYYY-MM-DD")
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

// class GoogleFlightsResponse {
//   best_flights ResponseFlights[]
// }
// class ResponseFlights {
//   flights ResponseFlight[]
// }
//
// class ResponseFlight {
//   departure_airport ResponseAirport
// }
//

function GoogleSearchFlights(departure_id: string, arrival_id: string, adults: int, children: int, outbound_date: string, return_date: string) -> string {
  let api_key = env.SERP_API_KEY;
  let resp = baml.fetch_as<string>(baml.HttpRequest {
      url: "https://serpapi.com/search?engine=google_flights&api_key=" + api_key + "&departure_id=" + departure_id + "&arrival_id=" + arrival_id + "&adults=" + baml.unstable.string(adults) + "&children=" + baml.unstable.string(children) + "&outbound_date=" + outbound_date + "&return_date=" + return_date,
      method: baml.HttpMethod.Get,
      headers: { }
    });
  resp
}

function LLMSumarizeFlights(flights: string) -> string {
  client CustomSonnet4
  prompt #"
    Summarize the following flights as a 1-liner:
    {{ flights }}

    {{ ctx.output_format }}
  "#
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

    Try to keep each message friendly but brief. First message,
    2 or 3 sentences. After that, preferably 1 sentence or question.

    Here is the full conversation history:
    {{ history }}
  "#
}

function Main() -> int {
  watch let debug = "";
  debug = "About to get API key";
  debug = env.SERP_API_KEY;
  debug = GoogleSearchFlights("IAD", "AUS", 1, 0, "2025-10-26", "2025-10-26");
  watch let messages: Message[] = [];
  watch let itinerary = Itinerary {
    flights: [],
    activities: []
  };
  watch let context = TravelAgentContext {
    nAdults: null,
    nChildren: null,
    interests: [],
    homeLocation: null,
    dateRange: null
  };
  watch let current_tool_type = "no tool yet";
  let n = 0;

  for (;;) {
     n += 1;
     if (n > 10) {
       break;
     }
	 let tools = LLMChooseTool(messages, context);
	 for (let t in tools) {
		if (t instanceof RequestInfoFromUser) {
		  current_tool_type = t.type;
		  debug = "Got RequestInfoFromUser";
		  messages.push(Message { sender: "agent", content: t.message_to_user });
		  let userResponse = TalkToUser(t.message_to_user);
		  messages.push(Message { sender: "user", content: userResponse} );
		} else if (t instanceof SetItinerary) {

		} else if (t instanceof UpdateTravelAgentContext) {
          context = t.new_context;
		} else if (t instanceof SearchFlights) {
		  let resp = GoogleSearchFlights(t.departure_id, t.arrival_id, t.adults, t.children, t.outbound_date, t.return_date);
		  debug = resp;
		  let summary = LLMSumarizeFlights(resp);
	    messages.push(Message { sender: "agent", content: summary });
		} else if (t instanceof SetItinerary) {
      debug = "*******Setting itinerary **********";
      itinerary = t.itinerary;
    } else {
		  debug = "FALLTHROUGH!!!";
		}
	 }
  }
  debug = "finished for somehow";

  0
}
