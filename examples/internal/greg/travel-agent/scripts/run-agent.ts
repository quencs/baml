import { b, watchers } from "../baml_client";

async function runAgent() {
  console.log("[Agent] Starting travel agent...");

  try {
    const watcher = watchers.Main();
    watcher.on_var("debug", (ev) => console.log(ev));
    console.log("AHHHHHHH");
    await b.Main({ watchers: watcher });
  } catch (error) {
    console.error("[Agent] Error running agent:", error);
    process.exit(1);
  }
}

runAgent();
