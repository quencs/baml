import { b } from "../test-setup";

describe("OpenAI Provider", () => {
  it("should support openai-responses basic", async () => {
    const res = await b.request.TestOpenAIResponses("lorem ipsum");
    expect(res.body.json()).toMatchObject({
      role: "asdf",
    });
  });

  it("should support openai-responses explicit", async () => {
    const res = await b.request.TestOpenAIResponsesExplicit("lorem ipsum");
    expect(res.body.json()).toMatchObject({
      role: "asdf",
    });
  });

  it("should support openai-responses custom url", async () => {
    const res = await b.request.TestOpenAIResponsesCustomURL("lorem ipsum");
    expect(res.body.json()).toMatchObject({
      role: "asdf",
    });
  });

  it("should support openai-responses conversation", async () => {
    const res = await b.request.TestOpenAIResponsesConversation("lorem ipsum");
    expect(res.body.json()).toMatchObject({
      role: "asdf",
    });
  });

  it("should support openai-responses different model", async () => {
    const res = await b.request.TestOpenAIResponsesDifferentModel("lorem ipsum");
    expect(res.body.json()).toMatchObject({
      role: "asdf",
    });
  });
});
