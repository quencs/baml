import { b } from "./baml_client";

async function demoStreamUsage() {
  // Create a stream
  const stream = b.stream.StreamTest();

  // CORRECT: Use 'for await...of' for async iteration
  for await (const partial of stream) {
    console.log("Partial:", partial);
  }

  // Get the final complete response
  const final = await stream.getFinalResponse();
  console.log("Final:", final);
}

// INCORRECT: This will throw an error
function incorrectUsage() {
  const stream = b.stream.StreamTest();
  
  // This will throw: BamlStream requires async iteration. Use "for await...of" instead of "for...of"
  // for (const partial of stream) {
  //   console.log(partial);
  // }
}

// Run the demo
demoStreamUsage().catch(console.error);