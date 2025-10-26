import { b, watchers } from "../baml_client";

async function sendContextUpdate(context: any) {
  try {
    await fetch("http://localhost:3001/api/watch/context", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(context),
    });
  } catch (error) {
    console.error("[Agent] Failed to send context update:", error);
  }
}

async function runAgent() {
  console.log("[Agent] Starting travel agent...");

  try {
    const watcher = watchers.Main();
    watcher.on_var("debug", (ev) => console.log("[Watch debug]", ev));
    watcher.on_var("context", (ev) => {
      console.log("[Watch context]", ev);
      sendContextUpdate(ev);
    });
    console.log("[Agent] Watchers configured");
    await b.Main({ watchers: watcher });
  } catch (error) {
    console.error("[Agent] Error running agent:", error);
    process.exit(1);
  }
}

runAgent();
