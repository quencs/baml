import { spawn } from "child_process";
import path from "path";

let agentProcess: ReturnType<typeof spawn> | null = null;

export function startAgent() {
  if (agentProcess) {
    console.log("[AgentRunner] Agent already running");
    return;
  }

  console.log("[AgentRunner] Starting BAML agent...");

  const scriptPath = path.join(process.cwd(), "scripts", "run-agent.ts");

  // Use tsx to run the TypeScript file directly
  agentProcess = spawn("npx", ["tsx", scriptPath], {
    stdio: ["inherit", "pipe", "pipe"],
    cwd: process.cwd(),
  });

  agentProcess.stdout?.on("data", (data) => {
    console.log(`[Agent] ${data.toString().trim()}`);
  });

  agentProcess.stderr?.on("data", (data) => {
    console.error(`[Agent Error] ${data.toString().trim()}`);
  });

  agentProcess.on("exit", (code, signal) => {
    console.log(`[AgentRunner] Agent exited with code ${code}, signal ${signal}`);
    agentProcess = null;

    // Restart the agent if it exits unexpectedly
    if (code !== 0 && signal !== "SIGTERM") {
      console.log("[AgentRunner] Restarting agent in 5 seconds...");
      setTimeout(startAgent, 5000);
    }
  });

  agentProcess.on("error", (error) => {
    console.error("[AgentRunner] Failed to start agent:", error);
    agentProcess = null;
  });

  console.log("[AgentRunner] Agent process started");
}

export function stopAgent() {
  if (agentProcess) {
    console.log("[AgentRunner] Stopping agent...");
    agentProcess.kill("SIGTERM");
    agentProcess = null;
  }
}

// Handle graceful shutdown
process.on("SIGINT", () => {
  stopAgent();
  process.exit(0);
});

process.on("SIGTERM", () => {
  stopAgent();
  process.exit(0);
});
