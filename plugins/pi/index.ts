import { execFile } from "node:child_process";

const OMNI_AGENT_ID = "pi";
const DEFAULT_OMNI_PATH = "omni";
const OMNI_TIMEOUT_MS = 10_000;
const OMNI_STDIN_LIMIT_BYTES = 16 * 1024 * 1024;
const MUTATION_TOOLS = new Set(["edit", "write"]);

type JsonObject = Record<string, unknown>;

type ExtensionAPI = {
  on(eventName: string, handler: (...args: unknown[]) => unknown): void;
};

type ExtensionContext = {
  sessionId?: string;
  cwd?: string;
  workingDirectory?: string;
  config?: { omniPath?: string };
  extensionConfig?: { omniPath?: string };
};

type SessionBeforeCompactEvent = {
  sessionId?: string;
  compactionReason?: string;
  reason?: string;
};

type BeforeAgentStartEvent = {
  systemPrompt: string;
};

type ToolResultEvent = {
  toolName: string;
  input?: unknown;
  output?: unknown;
  result?: unknown;
  response?: unknown;
  content?: unknown;
  isError?: boolean;
};

type OmniHookOutput = {
  hookSpecificOutput?: {
    systemPromptAddition?: string;
    updatedResponse?: string;
    additionalContext?: string;
  };
};

let pendingSystemPromptAddition: string | undefined;

function contextFromArgs(args: unknown[]): ExtensionContext | undefined {
  const objects = args.filter((arg): arg is ExtensionContext => {
    return typeof arg === "object" && arg !== null && !Array.isArray(arg);
  });

  return (
    objects.find((arg) => {
      return Boolean(
        arg.config ||
          arg.extensionConfig ||
          arg.cwd ||
          arg.workingDirectory ||
          arg.sessionId,
      );
    }) || objects.at(-1)
  );
}

function omniPathFromContext(ctx?: ExtensionContext): string {
  return ctx?.config?.omniPath || ctx?.extensionConfig?.omniPath || DEFAULT_OMNI_PATH;
}

function workingDirectoryFromContext(ctx?: ExtensionContext): string {
  return ctx?.workingDirectory || ctx?.cwd || process.cwd();
}

function sessionIdFromContext(ctx?: ExtensionContext): string {
  return ctx?.sessionId || `pi-${process.pid}`;
}

function bytesFor(value: string): number {
  return Buffer.byteLength(value, "utf8");
}

async function runOmni(
  args: string[],
  payload: JsonObject,
  ctx?: ExtensionContext,
): Promise<OmniHookOutput | undefined> {
  const stdin = JSON.stringify(payload);
  if (bytesFor(stdin) > OMNI_STDIN_LIMIT_BYTES) {
    return undefined;
  }

  return new Promise((resolve) => {
    const child = execFile(
      omniPathFromContext(ctx),
      args,
      {
        env: { ...process.env, OMNI_AGENT_ID },
        timeout: OMNI_TIMEOUT_MS,
        maxBuffer: OMNI_STDIN_LIMIT_BYTES,
      },
      (error, stdout) => {
        if (error || !stdout.trim()) {
          resolve(undefined);
          return;
        }

        try {
          resolve(JSON.parse(stdout) as OmniHookOutput);
        } catch {
          resolve(undefined);
        }
      },
    );

    child.stdin?.end(stdin);
  });
}

async function runSessionStart(ctx?: ExtensionContext): Promise<void> {
  const result = await runOmni(
    ["--session-start"],
    {
      hookEventName: "SessionStart",
      sessionId: sessionIdFromContext(ctx),
      workingDirectory: workingDirectoryFromContext(ctx),
    },
    ctx,
  );

  pendingSystemPromptAddition = result?.hookSpecificOutput?.systemPromptAddition || undefined;
}

async function runPreCompact(
  event: SessionBeforeCompactEvent,
  ctx?: ExtensionContext,
): Promise<void> {
  const result = await runOmni(
    ["--pre-compact"],
    {
      hookEventName: "PreCompact",
      sessionId: event.sessionId || sessionIdFromContext(ctx),
      compactionReason: event.compactionReason || event.reason || "context_limit_reached",
    },
    ctx,
  );

  pendingSystemPromptAddition = result?.hookSpecificOutput?.systemPromptAddition || undefined;
}

function toolNameForOmni(toolName: string | undefined): string {
  if (!toolName) {
    return "Unknown";
  }

  const normalized = toolName.trim();
  const lower = normalized.toLowerCase();

  switch (lower) {
    case "bash":
    case "shell":
    case "exec":
      return "Bash";
    case "ls":
      return "LS";
    case "read":
    case "read_file":
      return "Read";
    case "grep":
    case "search":
      return "Grep";
    case "webfetch":
    case "web_fetch":
    case "fetch":
      return "WebFetch";
    case "edit":
      return "Edit";
    case "write":
      return "Write";
    default:
      return normalized || toolName;
  }
}

function textFromUnknown(value: unknown): string {
  if (typeof value === "string") {
    return value;
  }
  if (Array.isArray(value)) {
    return value.map(textFromUnknown).filter(Boolean).join("\n");
  }
  if (typeof value === "object" && value !== null) {
    const obj = value as JsonObject;
    if (typeof obj.text === "string") {
      return obj.text;
    }
    if (typeof obj.content === "string") {
      return obj.content;
    }
    if (typeof obj.output === "string") {
      return obj.output;
    }
  }
  return "";
}

function toolResponseForOmni(event: ToolResultEvent): JsonObject {
  const rawOutput = event.response ?? event.result ?? event.output ?? event.content;
  const content = textFromUnknown(rawOutput);

  if (toolNameForOmni(event.toolName) === "Bash") {
    return event.isError ? { stderr: content } : { stdout: content };
  }

  return { content };
}

async function runPostTool(event: ToolResultEvent, ctx?: ExtensionContext): Promise<unknown> {
  if (!event?.toolName || MUTATION_TOOLS.has(event.toolName.toLowerCase())) {
    return undefined;
  }

  const result = await runOmni(
    ["--post-hook"],
    {
      tool_name: toolNameForOmni(event.toolName),
      tool_input: event.input ?? {},
      tool_response: toolResponseForOmni(event),
    },
    ctx,
  );

  const output = result?.hookSpecificOutput;
  const updatedResponse = output?.updatedResponse?.trim();
  if (!updatedResponse) {
    return undefined;
  }

  const text = [updatedResponse, output?.additionalContext].filter(Boolean).join("\n\n");
  return { content: [{ type: "text" as const, text }] };
}

export default function omniExtension(pi: ExtensionAPI): void {
  pi.on("session_start", async (...args: unknown[]) => {
    await runSessionStart(contextFromArgs(args));
  });

  pi.on("before_agent_start", async (rawEvent: unknown) => {
    if (!pendingSystemPromptAddition) {
      return undefined;
    }

    const event = rawEvent as BeforeAgentStartEvent;
    if (typeof event.systemPrompt !== "string") {
      return undefined;
    }

    const systemPrompt = `${event.systemPrompt}\n\n${pendingSystemPromptAddition}`;
    pendingSystemPromptAddition = undefined;
    return { systemPrompt };
  });

  pi.on("session_before_compact", async (...args: unknown[]) => {
    const event = (args[0] ?? {}) as SessionBeforeCompactEvent;
    await runPreCompact(event, contextFromArgs(args.slice(1)));
  });

  pi.on("tool_result", async (...args: unknown[]) => {
    const event = args[0] as ToolResultEvent;
    return runPostTool(event, contextFromArgs(args.slice(1)));
  });
}
