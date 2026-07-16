import { createServer } from "node:http";
import { Agent, Cursor } from "@cursor/sdk";

type ChatMessage = {
  role: string;
  content?: string | Array<{ type?: string; text?: string }>;
  tool_calls?: Array<{
    id: string;
    type?: string;
    function?: { name?: string; arguments?: string };
  }>;
  tool_call_id?: string;
  name?: string;
};

type ToolDef = {
  type?: string;
  function?: { name?: string; description?: string; parameters?: unknown };
};

type ChatRequest = {
  model?: string;
  messages?: ChatMessage[];
  tools?: ToolDef[];
  tool_choice?: unknown;
  stream?: boolean;
};

const PORT = Number(process.env.PORT || "0");
const HOST = process.env.HOST || "127.0.0.1";
const DEFAULT_MODEL = process.env.DEFAULT_MODEL || "grok-4.5";
const API_KEY = process.env.CURSOR_API_KEY || "";
const CWD = process.env.CURSOR_CWD || process.cwd();

type ModelSelection = {
  id: string;
  params?: Array<{ id: string; value: string }>;
};

/** Map OpenAI-style / legacy ids onto Cursor SDK ModelSelection. */
function resolveModel(raw: string | undefined): ModelSelection {
  const id = (raw || DEFAULT_MODEL).trim();
  // Legacy grok-build slug — Cursor SDK only accepts `grok-4.5` (+ params).
  if (
    id === "cursor-grok-4.5-high" ||
    id === "cursor-grok-4.5" ||
    id === "grok-4.5-cursor" ||
    id === "grok-4.5-high"
  ) {
    return {
      id: "grok-4.5",
      params: [
        { id: "effort", value: "high" },
        { id: "fast", value: "false" },
      ],
    };
  }
  if (id === "cursor-grok-4.5-high-fast" || id === "grok-4.5-fast") {
    return {
      id: "grok-4.5",
      params: [
        { id: "effort", value: "high" },
        { id: "fast", value: "true" },
      ],
    };
  }
  if (id === "grok-4.5") {
    return {
      id: "grok-4.5",
      params: [
        { id: "effort", value: "high" },
        { id: "fast", value: "false" },
      ],
    };
  }
  if (id === "auto" || id === "default") {
    return { id: "default" };
  }
  return { id };
}

function requireAuth(req: { headers: { authorization?: string } }): boolean {
  const auth = req.headers.authorization || "";
  const token = auth.startsWith("Bearer ") ? auth.slice(7).trim() : "";
  if (!API_KEY) return false;
  return token === API_KEY || token.length > 0;
}

function textOf(content: ChatMessage["content"]): string {
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return "";
  return content
    .map((part) => (typeof part?.text === "string" ? part.text : ""))
    .filter(Boolean)
    .join("\n");
}

function messagesToPrompt(messages: ChatMessage[]): string {
  const parts: string[] = [];
  for (const msg of messages) {
    const role = msg.role || "user";
    if (role === "tool") {
      parts.push(
        `Tool result (${msg.tool_call_id || msg.name || "tool"}):\n${textOf(msg.content)}`,
      );
      continue;
    }
    if (msg.tool_calls?.length) {
      const calls = msg.tool_calls
        .map((tc) => {
          const name = tc.function?.name || "unknown";
          const args = tc.function?.arguments || "{}";
          return `Call tool ${name} with ${args}`;
        })
        .join("\n");
      parts.push(`Assistant tool calls:\n${calls}`);
      continue;
    }
    const body = textOf(msg.content);
    if (!body) continue;
    parts.push(`${role}:\n${body}`);
  }
  return parts.join("\n\n");
}

function toolInstructions(tools: ToolDef[] | undefined): string {
  if (!tools?.length) return "";
  const lines = tools.map((t) => {
    const name = t.function?.name || "tool";
    const desc = t.function?.description || "";
    const params = JSON.stringify(t.function?.parameters ?? {});
    return `- ${name}: ${desc}\n  parameters: ${params}`;
  });
  return [
    "",
    "You are running behind an OpenAI-compatible bridge for Grok Build.",
    "Grok executes tools locally — you must NOT run shell/file tools yourself.",
    "When you need an action, emit ONLY a JSON object on its own line:",
    '{"tool_calls":[{"id":"call_1","type":"function","function":{"name":"TOOL_NAME","arguments":"{}"}}]}',
    "Available tools:",
    ...lines,
  ].join("\n");
}

function parseToolCalls(text: string): {
  content: string;
  tool_calls?: Array<{
    id: string;
    type: "function";
    function: { name: string; arguments: string };
  }>;
} {
  const match = text.match(/\{[\s\S]*"tool_calls"\s*:\s*\[[\s\S]*\][\s\S]*\}/);
  if (!match) return { content: text };
  try {
    const parsed = JSON.parse(match[0]) as {
      tool_calls?: Array<{
        id?: string;
        type?: string;
        function?: { name?: string; arguments?: string };
      }>;
    };
    if (!parsed.tool_calls?.length) return { content: text };
    const tool_calls = parsed.tool_calls
      .filter((tc) => tc.function?.name)
      .map((tc, i) => ({
        id: tc.id || `call_${i + 1}`,
        type: "function" as const,
        function: {
          name: tc.function!.name!,
          arguments:
            typeof tc.function!.arguments === "string"
              ? tc.function!.arguments
              : JSON.stringify(tc.function!.arguments ?? {}),
        },
      }));
    if (!tool_calls.length) return { content: text };
    const content = text.replace(match[0], "").trim();
    return { content, tool_calls };
  } catch {
    return { content: text };
  }
}

async function runAgent(
  model: string,
  messages: ChatMessage[],
  tools: ToolDef[] | undefined,
): Promise<{ content: string; tool_calls?: ReturnType<typeof parseToolCalls>["tool_calls"] }> {
  const prompt = `${messagesToPrompt(messages)}${toolInstructions(tools)}`;
  const selection = resolveModel(model);
  // ask mode: model replies without Cursor executing local tools — Grok owns tools.
  const agent = await Agent.create({
    apiKey: API_KEY,
    model: selection,
    local: { cwd: CWD },
    mode: "ask",
  });
  try {
    const run = await agent.send(prompt);
    const settled = await run.wait();
    if (settled.status === "error") {
      throw new Error(settled.error?.message || "Cursor run failed");
    }
    const text = (settled.result || "").trim();
    return parseToolCalls(text);
  } finally {
    agent.close();
  }
}

function isNonRetryableBridgeError(message: string): boolean {
  const m = message.toLowerCase();
  return (
    m.includes("cannot use this model") ||
    m.includes("available models") ||
    m.includes("unauthorized") ||
    m.includes("invalid api key") ||
    m.includes("authentication")
  );
}

function sseChunk(data: unknown): string {
  return `data: ${JSON.stringify(data)}\n\n`;
}

async function handleChat(
  body: ChatRequest,
  res: import("node:http").ServerResponse,
): Promise<void> {
  const model = body.model || DEFAULT_MODEL;
  const messages = body.messages || [];
  const tools = body.tools;
  const stream = Boolean(body.stream);
  const result = await runAgent(model, messages, tools);
  const id = `chatcmpl_${Date.now()}`;
  const created = Math.floor(Date.now() / 1000);

  if (!stream) {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(
      JSON.stringify({
        id,
        object: "chat.completion",
        created,
        model,
        choices: [
          {
            index: 0,
            message: {
              role: "assistant",
              content: result.content || null,
              tool_calls: result.tool_calls,
            },
            finish_reason: result.tool_calls?.length ? "tool_calls" : "stop",
          },
        ],
      }),
    );
    return;
  }

  res.writeHead(200, {
    "Content-Type": "text/event-stream",
    "Cache-Control": "no-cache",
    Connection: "keep-alive",
  });
  if (result.tool_calls?.length) {
    res.write(
      sseChunk({
        id,
        object: "chat.completion.chunk",
        created,
        model,
        choices: [
          {
            index: 0,
            delta: { role: "assistant", tool_calls: result.tool_calls },
            finish_reason: "tool_calls",
          },
        ],
      }),
    );
  } else {
    res.write(
      sseChunk({
        id,
        object: "chat.completion.chunk",
        created,
        model,
        choices: [
          {
            index: 0,
            delta: { role: "assistant", content: result.content },
            finish_reason: null,
          },
        ],
      }),
    );
    res.write(
      sseChunk({
        id,
        object: "chat.completion.chunk",
        created,
        model,
        choices: [{ index: 0, delta: {}, finish_reason: "stop" }],
      }),
    );
  }
  res.write("data: [DONE]\n\n");
  res.end();
}

async function listModels(): Promise<unknown[]> {
  try {
    const listed = await Cursor.models.list({ apiKey: API_KEY });
    if (Array.isArray(listed)) {
      return listed.map((m: { id?: string } | string) => {
        const id = typeof m === "string" ? m : m.id || DEFAULT_MODEL;
        return { id, object: "model", created: 0, owned_by: "cursor" };
      });
    }
  } catch {
    // fall through
  }
  return [
    { id: "grok-4.5", object: "model", created: 0, owned_by: "cursor" },
    { id: "composer-2.5", object: "model", created: 0, owned_by: "cursor" },
    { id: "default", object: "model", created: 0, owned_by: "cursor" },
    { id: "auto", object: "model", created: 0, owned_by: "cursor" },
  ];
}

if (!API_KEY) {
  console.error("CURSOR_API_KEY is required");
  process.exit(1);
}

const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url || "/", `http://${HOST}`);
    if (req.method === "GET" && url.pathname === "/health") {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ ok: true, default_model: DEFAULT_MODEL }));
      return;
    }
    if (!requireAuth(req)) {
      res.writeHead(401, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ error: { message: "Unauthorized" } }));
      return;
    }
    if (req.method === "GET" && url.pathname === "/v1/models") {
      const data = await listModels();
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ object: "list", data }));
      return;
    }
    if (req.method === "POST" && url.pathname === "/v1/chat/completions") {
      const chunks: Buffer[] = [];
      for await (const chunk of req) chunks.push(chunk as Buffer);
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}") as ChatRequest;
      await handleChat(body, res);
      return;
    }
    res.writeHead(404, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ error: { message: "Not found" } }));
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error("bridge error:", message);
    // 400 for bad model / auth so Grok does not spin "Retrying…" on a permanent error.
    const status = isNonRetryableBridgeError(message) ? 400 : 500;
    if (!res.headersSent) {
      res.writeHead(status, { "Content-Type": "application/json" });
    }
    res.end(JSON.stringify({ error: { message } }));
  }
});

server.listen(PORT, HOST, () => {
  const addr = server.address();
  if (addr && typeof addr === "object") {
    console.log(`CURSOR_BRIDGE_READY ${HOST}:${addr.port}`);
  }
});
