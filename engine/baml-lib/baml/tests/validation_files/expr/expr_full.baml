function MakePoem(length: int) -> string {
    client GPT4o
    prompt #"Write a poem {{ length }} lines long."#
}

function CombinePoems(poem1: string, poem2: string) -> string {
    client GPT4o
    prompt #"Combine the following two poems into one poem.

    Poem 1:
    {{ poem1 }}

    Poem 2:
    {{ poem2 }}
    "#
}

let poem = MakePoem(10);

let another = {
  let x = MakePoem(10);
  let y = MakePoem(5);
  CombinePoems(x,y)
};

function Pipeline() -> string {
    let x = MakePoem(6);
    let y = MakePoem(6);
    let a = MakePoem(6);
    let b = MakePoem(6);
    let xy = CombinePoems(x,y);
    let ab = CombinePoems(a,b);
    CombinePoems(xy, ab)
}

function Pyramid() -> string {
  CombinePoems( CombinePoems( MakePoem(10), MakePoem(10)), MakePoem(10))
}

function OuterPyramid() -> string {
  CombinePoems(poem, another)
}

test TestPipeline() {
  functions [Pipeline]
  args { }
}

test TestPyramid() {
  functions [Pyramid]
  args { }
}

test OuterPyramid() {
  functions [OuterPyramid]
  args { }
}

client<llm> GPT4o {
  provider openai
  options {
    model gpt-4o
    api_key env.OPENAI_API_KEY
  }
}

test TestMakePoem() {
    functions [MakePoem]
    args { length 4 }
}

function Search(search_term: string) -> string[] {
  client GPT4o
  prompt #"
    Search for {{ search_term }}
  "#
}

function AgentLoop(search_term: string) -> string[] {
  let all_terms = [];
  let max_terms = 10;
  while (all_terms.length < 10 && max_terms > 0) {
    let search_results = Search(search_term);
    let terms = search_results.filter((result) => {
      // ensure each result is a high qualty result and related to the search term
      let quality = EnsureQuality(search_term, result);
      if (quality.score > 0.5) {
        return true;
      }
    })
    all_terms.extend(terms);
    max_terms -= 1;
  }
  all_terms
}