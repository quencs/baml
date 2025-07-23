// Defining a data model.
class Query {
  text string
  language_preference string? @description("The user is building their BAML application with this language")
}

class RefineQuery {
  reason string @description("Why is the query too vague?")

  suggested_queries string[] @description("Suggestions for how to improve the query")
}

class SearchDocumentation {
  @@description("User is asking about a specific topic, we should search the documentation for the answer")

  reason string @description("Why is the query about a specific topic?")

  // search_context string[] @description("Additional context we should add to the search")
}

class HelpWritingCode {
  @@description("User is asking for help writing or debugging code")

  help_with BamlLanguage
}

enum BamlLanguage {
  Baml
  Python
  Typescript
  Javascript
  Ruby
  Go
  Other
}

class QueryActionPlan {

  refine_query RefineQuery?
  @description("Query is impossibly vague, user needs to be more specific about what they're asking about")

  search_documentation SearchDocumentation?

  help_writing_code HelpWritingCode?
}

// Create a function to extract the resume from a string.
function PlanQuery(query: Query) -> QueryActionPlan {
  // Specify a client as provider/model-name
  // you can use custom LLM params with a custom client name from clients.baml like "client CustomHaiku"
  client "openai/gpt-4o" // Set OPENAI_API_KEY to use this client.
  prompt #"
    {{ _.role("system") }}
    You will be answering questions about BAML (Basically, a Made-up Language) -- an AI framework to write LLM prompts as functions. Here is some background:
    <Overview>
    BAML (Basically, A Made-Up Language) is a domain-specific language for building LLM prompts as functions.
    You can build an agentic workflow with BAML.
    </Overview>

    <Schema>
    // Define output schemas using classes
    class MyObject {
      // Optional string fields use ?
      // @description is optional, but if you include it, it goes after the field.
      name string? @description("The name of the object")

      // Arrays of primitives
      // arrays cannot be optional.
      tags string[]

      // Enums must be declared separately and are optional
      status MyEnum?

      // Union types
      type "success" | "error"

      // Primitive types
      count int
      enabled bool
      score float

      // nested objects
      nested MyObject2

      // image type
      myImg image
    }

    // Enums are declared separately
    enum MyEnum {
      // enum values MUST START WITH A CAPITAL LETTER
      PENDING
      ACTIVE @description("Item is currently active")
      COMPLETE
    }

    // Comments use double slashes
    // Recursive types and inline definitions are not supported
    </Schema>

    <Functions>
    // Functions define inputs, outputs and prompts
    // function name is always PascalCase
    function MyFunction(input: MyObject) -> string {
      client "openai/gpt-4o"
      // prompt with jinja syntax inside here. with double curly braces for variables.
      // make sure to include: \{\{ ctx.output_format \}\} in the prompt, which prints the output schema instructions so the LLM returns the output in the correct format (json or string, etc.). DO NOT write the output schema manually.
      prompt #"

      \"\#
    }

    <LLMClients>
      You can use any of the following if you write a baml function example:
      - openai/gpt-4o
      - openai/gpt-4o-mini
      - anthropic/claude-3-5-sonnet-latest (note the "3-5")
      - anthropic/claude-3-5-haiku-latest

      If the user mentions an open-source model like "llama3" or "r1", search the docs for "OpenRouter" or "openai-generic" to see how to use it.
    </LLMClients>

    <Prompt>
      In case you need to write a prompt example:
      1. Make sure to include the input in the prompt (even if it's an image) using \{\{ input \}\}
      2. Make sure to include \{\{ ctx.output_format \}\} in the prompt so the LLM knows how to format the output.
      3. You do not need to specify to "answer in JSON format". Only write in the prompt brief instruction, and any other task-specific things to keep in mind for the task.
      4. Write a \{\{ _.role("user") \}\} tag to indicate where the user's inputs start. So if there's a convo you can write
      \{\{ _.role("user") \}\} \{\{ some-variable \}\}

      DO NOT REPEAT output schema fields in the prompt. They are included with \{\{ "\{\{ ctx.output_format \}\}" \}\}.
      ```baml
      class TweetAnalysis {
        mainTopic string @description("The primary topic or subject matter of the tweet")
        isSpam bool @description("Whether the tweet appears to be spam")
      }

      function ClassifyTweets(tweets: string[]) -> TweetAnalysis[] {
        client "openai/gpt-4o-mini"
        prompt #"
          Analyze each of the following tweets and classify them.

          \{\{ ctx.output_format \}\}

          \{\{ _.role("user") \}\} \{\{ tweets \}\}
        \"\#
      }
    ```
    </Prompt>
    </Functions>

    <Usage in other languages>
    You can use BAML in python, typescript, and other languages.

    ```python
    import asyncio
    from baml_client import b // this client is autogenerated
    from baml_client.types import WeatherAPI

    def main():
        # In python, BAML functions are synchronous.
        weather_info = b.UseTool("What's the weather like in San Francisco?")
        print(weather_info)
        assert isinstance(weather_info, WeatherAPI)
        print(f"City: {weather_info.city}")
        print(f"Time of Day: {weather_info.timeOfDay}")

    if __name__ == '__main__':
        main()
    ```

    ```typescript
    import { b } from './baml_client' // this client is autogenerated
    import { WeatherAPI } from './baml_client/types'
    import assert from 'assert'

    const main = async () => {
      const weatherInfo = await b.UseTool("What's the weather like in San Francisco?")
      console.log(weatherInfo)
      assert(weatherInfo instanceof WeatherAPI)
      console.log(`City: ${weatherInfo.city}`)
      console.log(`Time of Day: ${weatherInfo.timeOfDay}`)
        }
    ```
    </Usage>

    <Tool Use>
    Here's an example of how tools work in BAML:

    1. Define tool classes that represent the structured output you want
    2. Create a function that returns one of those tool classes
    3. The function's prompt will automatically format user messages and add output validation

    The LLM will choose which tool to call, and your python code will handle the tool call with the LLM's chosen parameters.

    ```baml
    // Tool classes define the structure of possible outputs
    class ChatMessage {
      role "user" | "assistant"
      content string
    }

    class WeatherAPI {
      city string @description("the user's city")
      timeOfDay string @description("As an ISO8601 timestamp")
    }

    // Function that can return different tool classes
    function UseTool(user_messages: ChatMessage[]) -> WeatherAPI | MyOtherAPI {
      client CustomSonnet
      prompt #"
        You are a helpful assistant that can use different tools to help users. Based on their message, choose the appropriate tool to use.

        \{\{ "\{\{ ctx.output_format \}\}" \}\}

        User message:
        \{\% for message in .....(jinja iterator) \%\}
        \{\{ _.role(message.role) \}\} \{\{ message.content \}\}
        \{\% endfor \%\}
      \"\#
    }
    ```

    ```python
    import asyncio
    from baml_client import b
    from baml_client.types import WeatherAPI, MyOtherAPI

    async def main():
        tool = b.UseTool("What's the weather like in San Francisco?")
        print(tool)

        if isinstance(tool, WeatherAPI):
            print(f"Weather API called:")
            print(f"City: {tool.city}")
            print(f"Time of Day: {tool.timeOfDay}")
        elif isinstance(tool, MyOtherAPI):
            print(f"MyOtherAPI called:")
            # Handle MyOtherAPI specific attributes here

    if __name__ == '__main__':
        main()
    ```
    </Tool Use>

    <Collecting usage>
    ```python
    from baml_client import b
    from baml_py import Collector

    # Create a collector with optional name
    collector = Collector(name="my-collector")

    # Use it with a function call
    result = b.ExtractResume("...", baml_options={"collector": collector})

    # Access logging information
    print(collector.last.usage)  # Print usage metrics
    print(collector.last.raw_llm_response)  # Print final response as string
    # since there may be retries, print the last http response received
    print(collector.last.calls[-1].http_response)
    ```

    ```typescript
    import { b } from 'baml_client'
    import { Collector } from '@boundaryml/baml'

    // Create a collector with optional name
    const collector = new Collector("my-collector")

    // Use it with a function call
    const result = await b.ExtractResume("...", { collector })

    // Access logging information
    console.log(collector.last?.usage)  // Print usage metrics
    console.log(collector.last?.rawLlmResponse)  // Print final response
    // since there may be retries, print the last http response received
    console.log(collector.last?.calls[-1].httpResponse)

    ```

    Users can define LLM clients and types dynamically at runtime (in python) as well using the "Client Registry" feature or the Dynamic Type feature.

    ---
    If you are not sure of the answer (you're not sure how to write the code), you can tell the user to join the Discord server and ask the question there (we respond very fast)

    {{ _.role("user") }}
    {{ query.text }}

    My language preference is:
    {{ query.language_preference }}

    {{ ctx.output_format }}
  "#
}



// Test the function with a sample resume. Open the VSCode playground to run this.
test query_plan {
  functions [PlanQuery]
  args {
    query {
      text #"how to call openai"#
      language_preference #"Python"#
    }
  }
}

test query_plan1 {
  functions [PlanQuery]
  args {
    query {
      text #"
        When I use baml in main why do my categories get to be any name but when I'm in test I have to use PascalCase?

        class Category(BaseModel):
            name: str
            embedding_text: str
            llm_description: str

        def _load_categories(csv_path: str) -> list[Category]:
            df = pd.read_csv(csv_path)
            categories = [
                Category(
                    name=row['name'],
                    embedding_text=row['embedding_text'],
                    llm_description=row['llm_description']
                )
                for _, row in df.iterrows()
      "#
      language_preference #"Python"#
    }
  }
}

test query_plan2 {
  functions [PlanQuery]
  args {
    query {
      text #"
        Help me understand this code:

        def _pick_best_categories(text: str, categories: list[Category]) -> list[Category]:
            tb = TypeBuilder()
            for k in categories:
                val = tb.Category.add_value(k.name)
                val.description(k.llm_description)
            selected_categories = b.PickBestCategories(text, count=3, baml_options={ "tb": tb })
            return [category for category in categories if category.name in selected_categories]
      "#
      language_preference #"Python"#
    }
  }
}