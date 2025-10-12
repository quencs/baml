// Used to test connection pool concurrency

const http = require("http");
const { URL } = require("url");

// Get host and port.
const HOST = getArg("--host") || process.env.HOST || "127.0.0.1";
const PORT = Number(getArg("--port") || process.env.PORT || 8001);

// Latency in milliseconds.
const LATENCY = Number(getArg("--latency") || process.env.LATENCY || 50);

// Get CLI args.
function getArg(flag) {
    const i = process.argv.indexOf(flag);
    return i !== -1 ? process.argv[i + 1] : undefined;
}

// Sleep millis.
function sleep(ms) {
    return new Promise((res) => setTimeout(res, ms));
}

// Respond with JSON.
function json(res, status, bodyObj) {
    const body = JSON.stringify(bodyObj);
    res.writeHead(status, {
        "Content-Type": "application/json",
        "Content-Length": Buffer.byteLength(body),
        "Cache-Control": "no-store",
        "Connection": "keep-alive",
        // CORS (harmless if you curl)
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Headers": "Content-Type, Authorization",
    });
    res.end(body);
}

async function handleRequest(req, res) {
    const url = new URL(req.url, `http://${req.headers.host}`);

    // Health
    if (req.method === "GET" && url.pathname === "/health") {
        return json(res, 200, { ok: true });
    }

    // OpenAI generic.
    if (req.method === "POST" && url.pathname === "/v1/chat/completions") {
        let body = "";

        req.on("data", chunk => body += chunk);

        req.on("end", async () => {
            // We don't actually need the request payload for this test.
            // But parse if present to avoid client errors.
            try {
                if (body && body.length) {
                    JSON.parse(body);
                }
            } catch {
                return json(res, 400, { error: { message: "Invalid JSON" } });
            }

            // Simulate latency for concurrency testing
            await sleep(LATENCY);

            const now = Math.floor(Date.now() / 1000);

            return json(res, 200, {
                id: `cmpl-${now}-${Math.random().toString(36).slice(2, 8)}`,
                object: "chat.completion",
                created: now,
                model: "concurrency-test",
                choices: [
                    {
                        index: 0,
                        message: { role: "assistant", content: "OpenAI" },
                        finish_reason: "stop",
                    },
                ],
                usage: { prompt_tokens: 0, completion_tokens: 1, total_tokens: 1 },
            });
        });

        return;
    }

    // Anthropic.
    if (req.method === "POST" && url.pathname === "/v1/messages") {
        let body = "";

        req.on("data", chunk => body += chunk);

        req.on("end", async () => {
            // We don't actually need the request payload for this test.
            // But parse if present to avoid client errors.
            try {
                if (body && body.length) {
                    JSON.parse(body);
                }
            } catch {
                return json(res, 400, { error: { message: "Invalid JSON" } });
            }

            // Simulate latency for concurrency testing
            await sleep(LATENCY);

            const now = Math.floor(Date.now() / 1000);

            return json(res, 200, {
                id: `msg_${Math.random().toString(36).slice(2, 10)}`,
                type: "message",
                role: "assistant",
                model: "concurrency-test",
                content: [
                    { type: "text", text: "Anthropic" }
                ],
                stop_reason: "end_turn",
                stop_sequence: null,
                usage: { input_tokens: 0, output_tokens: 1 },
                created_at: now,
            });
        });

        return;
    }

    // Not found
    json(res, 404, { error: { message: "Not found" } });
}

const server = http.createServer(async (req, res) => {
    console.log(`${req.method} ${req.url}`);

    try {
        await handleRequest(req, res);
    } catch (e) {
        json(res, 500, { error: { message: e?.message || "Internal error" } });
    }
});

server.listen(PORT, HOST, () => {
    process.stdout.write(`Concurrency test server listening on http://${HOST}:${PORT}\n`);
});
