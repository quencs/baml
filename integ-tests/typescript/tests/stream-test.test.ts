import { b } from "../baml_client";

describe("StreamTest", () => {
  it("should support for-await-of iteration", async () => {
    const stream = b.stream.StreamTest();
    
    const partials: string[] = [];
    for await (const partial of stream) {
      partials.push(partial);
      // Early exit after collecting some partials for test speed
      if (partials.length >= 3) break;
    }
    
    const final = await stream.getFinalResponse();
    
    expect(partials.length).toBeGreaterThan(0);
    // The final response should contain all the content
    expect(final.length).toBeGreaterThan(0);
  });

  it("should throw helpful error with regular for-of loop", () => {
    const stream = b.stream.StreamTest();
    
    // This should throw a helpful error message
    expect(() => {
      // The TypeScript compiler will show an error here, but we can still test the runtime behavior
      // by casting to any to bypass the type check
      for (const partial of stream as any) {
        console.log(partial);
      }
    }).toThrow('BamlStream requires async iteration. Use "for await...of" instead of "for...of"');
  });
});