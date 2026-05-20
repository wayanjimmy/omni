import { execFile } from "node:child_process";
import type {
  ExtensionAPI,
  ExtensionContext,
  SessionStartEvent,
  BeforeAgentStartEvent,
  SessionBeforeCompactEvent,
  ToolResultEvent,
} from "@earendil-works/pi-coding-agent";

const OMNI_AGENT_ID = "pi";
const DEFAULT_OMNI_PATH = "omni";
const OMNI_TIMEOUT_MS = 10_000;
const OMNI_STDIN_LIMIT_BYTES = 16 * 1024 * 1024;
const MUTATION_TOOLS = new Set(["edit", "write"]);
type JsonObject = Record<string, unknown>;

type OmniHookOutput = {
  hookSpecificOutput?: {
    systemPromptAddition?: string;
    updatedResponse?: string;
    additionalContext?: string;
  };
};

let pendingSystemPromptAddition: string | undefined;

/**
 * Find the ExtensionContext from the args array passed to event handlers.
 * Real ExtensionContext always has `cwd: string` and `ui`, so we use those
 * to identify it among the positional args.
 */
function contextFromArgs(args: unknown[]): ExtensionContext | undefined {
  return args.find((arg): arg is ExtensionContext => {
    return (
      typeof arg === "object" &&
      arg !== null &&
      !Array.isArray(arg) &&
      "cwd" in arg &&
      "ui" in arg
    );
  });
}

function omniPathFromContext(ctx?: ExtensionContext): string {
  // Look for omni path in extension config (stored as a custom property on context)
  const ctxObj = ctx as unknown as Record<string, unknown>;
  if (ctxObj._omniPath && typeof ctxObj._omniPath === "string") {
    return ctxObj._omniPath;
  }
  // Check extension-specific config if available
  const config = ctxObj.config as JsonObject | undefined;
  if (config?.omniPath && typeof config.omniPath === "string") {
    return config.omniPath;
  }
  const extCfg = ctxObj.extensionConfig as JsonObject | undefined;
  if (extCfg?.omniPath && typeof extCfg.omniPath === "string") {
    return extCfg.omniPath;
  }
  return DEFAULT_OMNI_PATH;
}

function workingDirectoryFromContext(ctx: ExtensionContext): string {
  return ctx.cwd;
}

function sessionIdFromManager(ctx: ExtensionContext): string {
  try {
const id = ctx.sessionManager.getSessionId();
    return id || "unknown";
  } catch {
    return "unknown";
  }
}

function bytesFor(value: string): number {
  return Buffer.byteLength(value, "utf8");
}

function runOmni(
  extraArgs: string[],
  stdin: JsonObject,
  ctx?: ExtensionContext,
): Promise<OmniHookOutput | undefined> {
  return new Promise((resolve) => {
    const omniPath = omniPathFromContext(ctx);
    const stdinJson = JSON.stringify(stdin);

    if (bytesFor(stdinJson) > OMNI_STDIN_LIMIT_BYTES) {
      resolve(undefined);
      return;
    }

    const args = ["--stdin", "--agent-id", OMNI_AGENT_ID, ...extraArgs];
    const cwd = ctx ? workingDirectoryFromContext(ctx) : process.cwd();

    const child = execFile(
      omniPath,
      args,
      {
        cwd,
        timeout: OMNI_TIMEOUT_MS,
        maxBuffer: OMNI_STDIN_LIMIT_BYTES,
        env: { ...process.env },
      },
      (error, stdout, stderr) => {
        if (error) {
          resolve(undefined);
          return;
        }

        try {
          const parsed = JSON.parse(stdout) as OmniHookOutput;
          resolve(parsed);
        } catch {
          resolve(undefined);
        }
      },
    );

    if (child.stdin) {
      child.stdin.write(stdinJson);
      child.stdin.end();
    }
  });
}

async function runOmniForSessionStart(
  event: SessionStartEvent,
  ctx: ExtensionContext,
): Promise<void> {
  await runOmni(
    ["--session-start"],
    {
      hookEventName: "SessionStart",
      sessionId: sessionIdFromManager(ctx),
      reason: event.reason,
    },
    ctx,
  );
}

async function runOmniForBeforeAgentStart(
  event: BeforeAgentStartEvent,
  ctx: ExtensionContext,
): Promise<void> {
  const result = await runOmni(
    ["--before-agent-start"],
    {
      hookEventName: "BeforeAgentStart",
      sessionId: sessionIdFromManager(ctx),
      systemPromptLength: event.systemPrompt.length,
      mutationTools: Array.from(MUTATION_TOOLS),
    },
    ctx,
  );

  pendingSystemPromptAddition =
    result?.hookSpecificOutput?.systemPromptAddition || undefined;
}

async function runOmniForPreCompact(
  event: SessionBeforeCompactEvent,
  ctx: ExtensionContext,
): Promise<void> {
  const result = await runOmni(
    ["--pre-compact"],
    {
      hookEventName: "PreCompact",
      sessionId: sessionIdFromManager(ctx),
      compactionReason: event.customInstructions || "context_limit_reached",
    },
    ctx,
  );

  pendingSystemPromptAddition =
    result?.hookSpecificOutput?.systemPromptAddition || undefined;
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
  const text = event.content
    .map((c) => {
      if (typeof c === "string") return c;
      if (c && typeof c === "object" && "type" in c && c.type === "text") {
        return (c as { text: string }).text;
      }
      return "";
    })
    .filter(Boolean)
    .join("\n");

  return {
    toolName: toolNameForOmni(event.toolName),
    result: text,
    isError: event.isError,
  };
}

export default function omniExtension(pi: ExtensionAPI): void {
  pi.on("session_start", async (...args: unknown[]) => {
    const ctx = contextFromArgs(args);
    if (!ctx) {
      return;
    }

    const event = args.find(
      (arg): arg is SessionStartEvent =>
        typeof arg === "object" &&
        arg !== null &&
        "type" in arg &&
        arg.type === "session_start",
    );

    if (!event) {
      return;
    }

    try {
      await runOmniForSessionStart(event, ctx);
    } catch {
      // OMNI fails silently — never crash the host
    }
  });

  pi.on("before_agent_start", async (...args: unknown[]) => {
    const ctx = contextFromArgs(args);
    if (!ctx) {
      return;
    }

    const event = args.find(
      (arg): arg is BeforeAgentStartEvent =>
        typeof arg === "object" &&
        arg !== null &&
        "type" in arg &&
        arg.type === "before_agent_start",
    );

    if (!event) {
      return;
    }

    try {
      await runOmniForBeforeAgentStart(event, ctx);
    } catch {
      // OMNI fails silently — never crash the host
    }
  });

  pi.on("session_before_compact", async (...args: unknown[]) => {
    const ctx = contextFromArgs(args);
    if (!ctx) {
      return;
    }

    const event = args.find(
      (arg): arg is SessionBeforeCompactEvent =>
        typeof arg === "object" &&
        arg !== null &&
        "type" in arg &&
        arg.type === "session_before_compact",
    );

    if (!event) {
      return;
    }

    try {
      await runOmniForPreCompact(event, ctx);
    } catch {
      // OMNI fails silently — never crash the host
    }
  });

  pi.on("tool_result", async (...args: unknown[]) => {
    const ctx = contextFromArgs(args);
    if (!ctx) {
      return;
    }

    const event = args.find(
      (arg): arg is ToolResultEvent =>
        typeof arg === "object" &&
        arg !== null &&
        "type" in arg &&
        arg.type === "tool_result",
    );

    if (!event) {
      return;
    }

    const toolName = event.toolName;
    if (!MUTATION_TOOLS.has(toolName)) {
      return;
    }

    try {
      await runOmni(
        ["--tool-result"],
        {
          hookEventName: "ToolResult",
          sessionId: sessionIdFromManager(ctx),
          toolName: toolNameForOmni(toolName),
          toolResponse: toolResponseForOmni(event),
          isError: event.isError,
        },
        ctx,
      );
    } catch {
      // OMNI fails silently — never crash the host
    }
  });
}
