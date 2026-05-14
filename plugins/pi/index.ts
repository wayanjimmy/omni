import { execFile } from "node:child_process";

const OMNI_AGENT_ID = "pi";
const DEFAULT_OMNI_PATH = "omni";
const OMNI_TIMEOUT_MS = 10_000;
const OMNI_STDIN_LIMIT_BYTES = 16 * 1024 * 1024;
const MUTATION_TOOLS = new Set(["edit", "write"]);
const COMMAND_KEY = "omni";

const DEFAULT_ENABLED = readBooleanEnv("PI_OMNI_ENABLED", true);
const SHOW_STATUS = readBooleanEnv("PI_OMNI_SHOW_STATUS", true);
const VERBOSE = readBooleanEnv("PI_OMNI_VERBOSE", false);

type JsonObject = Record<string, unknown>;

type CommandHandler = (args: string | undefined, ctx: CommandContext) => unknown;

type ExtensionAPI = {
  on(eventName: string, handler: (...args: unknown[]) => unknown): void;
  registerCommand?: (name: string, command: { description?: string; handler: CommandHandler }) => void;
};

type CommandContext = {
  sessionId?: string;
  ui: {
    notify(message: string, level?: "info" | "warning" | "error"): void;
    setStatus?(key: string, value: string): void;
  };
};

type ExtensionContext = {
  sessionId?: string;
  cwd?: string;
  workingDirectory?: string;
  config?: { omniPath?: string };
  extensionConfig?: { omniPath?: string };
  ui?: CommandContext["ui"];
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

const pendingSystemPromptAdditionBySession = new Map<string, string>();
const sessionEnabledBySession = new Map<string, boolean>();
let omniAvailable: boolean | null = null;

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

function isEnabled(sessionId: string): boolean {
  return sessionEnabledBySession.get(sessionId) ?? DEFAULT_ENABLED;
}

function setEnabled(sessionId: string, enabled: boolean): void {
  sessionEnabledBySession.set(sessionId, enabled);
  if (!enabled) {
    pendingSystemPromptAdditionBySession.delete(sessionId);
  }
}

async function probeOmni(ctx?: ExtensionContext, force = false): Promise<boolean> {
  if (!force && omniAvailable !== null) {
    return omniAvailable;
  }

  const result = await runOmniRaw(["--version"], "", ctx);
  omniAvailable = Boolean(result?.trim());
  return omniAvailable;
}

async function runOmniRaw(args: string[], stdin: string, ctx?: ExtensionContext): Promise<string | undefined> {
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
        resolve(stdout);
      },
    );

    child.stdin?.end(stdin);
  });
}

async function runOmni(
  args: string[],
  payload: JsonObject,
  ctx?: ExtensionContext,
): Promise<OmniHookOutput | undefined> {
  const sid = sessionIdFromContext(ctx);
  if (!isEnabled(sid)) {
    return undefined;
  }

  const available = await probeOmni(ctx);
  if (!available) {
    return undefined;
  }

  const stdin = JSON.stringify(payload);
  if (bytesFor(stdin) > OMNI_STDIN_LIMIT_BYTES) {
    return undefined;
  }

  const stdout = await runOmniRaw(args, stdin, ctx);
  if (!stdout) {
    return undefined;
  }

  try {
    return JSON.parse(stdout) as OmniHookOutput;
  } catch {
    return undefined;
  }
}

async function runSessionStart(ctx?: ExtensionContext): Promise<void> {
  const sid = sessionIdFromContext(ctx);
  if (!sessionEnabledBySession.has(sid)) {
    sessionEnabledBySession.set(sid, DEFAULT_ENABLED);
  }

  const result = await runOmni(
    ["--session-start"],
    {
      hookEventName: "SessionStart",
      sessionId: sid,
      workingDirectory: workingDirectoryFromContext(ctx),
    },
    ctx,
  );

  const addition = result?.hookSpecificOutput?.systemPromptAddition?.trim();
  if (addition) {
    pendingSystemPromptAdditionBySession.set(sid, addition);
  }
}

async function runPreCompact(
  event: SessionBeforeCompactEvent,
  ctx?: ExtensionContext,
): Promise<void> {
  const sid = event.sessionId || sessionIdFromContext(ctx);
  const result = await runOmni(
    ["--pre-compact"],
    {
      hookEventName: "PreCompact",
      sessionId: sid,
      compactionReason: event.compactionReason || event.reason || "context_limit_reached",
    },
    ctx,
  );

  const addition = result?.hookSpecificOutput?.systemPromptAddition?.trim();
  if (addition) {
    pendingSystemPromptAdditionBySession.set(sid, addition);
  }
}

function toolNameForOmni(toolName: string | undefined): string {
  if (!toolName) return "Unknown";
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
  if (typeof value === "string") return value;
  if (Array.isArray(value)) return value.map(textFromUnknown).filter(Boolean).join("\n");
  if (typeof value === "object" && value !== null) {
    const obj = value as JsonObject;
    if (typeof obj.text === "string") return obj.text;
    if (typeof obj.content === "string") return obj.content;
    if (typeof obj.output === "string") return obj.output;
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
  if (!updatedResponse) return undefined;

  const text = [updatedResponse, output?.additionalContext].filter(Boolean).join("\n\n");
  return { content: [{ type: "text" as const, text }] };
}

function statusLine(sessionId: string): string {
  const enabled = isEnabled(sessionId) ? "on" : "off";
  const availability = omniAvailable === null ? "unknown" : omniAvailable ? "available" : "missing";
  return `OMNI:${enabled} (${availability})`;
}

function notifyStatus(ctx: CommandContext, sessionId: string, forceNotify = false): void {
  const message = statusLine(sessionId);
  ctx.ui.setStatus?.("omni", message);
  if (forceNotify || SHOW_STATUS) {
    ctx.ui.notify(message, "info");
  }
}

function registerOmniCommand(pi: ExtensionAPI): void {
  if (!pi.registerCommand) {
    return;
  }

  const handler: CommandHandler = async (args, ctx) => {
    const sid = ctx.sessionId || `pi-${process.pid}`;
    const rawArgs = typeof args === "string" ? args : "";
    const [subRaw] = rawArgs.trim().split(/\s+/).filter(Boolean);
    const sub = (subRaw ?? "status").toLowerCase();

    switch (sub) {
      case "on":
        setEnabled(sid, true);
        await probeOmni({ sessionId: sid }, false);
        notifyStatus(ctx, sid, true);
        return;
      case "off":
        setEnabled(sid, false);
        notifyStatus(ctx, sid, true);
        return;
      case "refresh":
        await probeOmni({ sessionId: sid }, true);
        notifyStatus(ctx, sid, true);
        return;
      case "help":
        ctx.ui.notify("/omni status|on|off|refresh\nEnv: PI_OMNI_ENABLED, PI_OMNI_SHOW_STATUS, PI_OMNI_VERBOSE", "info");
        return;
      case "status":
      default:
        await probeOmni({ sessionId: sid }, false);
        notifyStatus(ctx, sid, true);
    }
  };

  pi.registerCommand(COMMAND_KEY, {
    description: "Manage OMNI distillation: /omni [status|on|off|refresh|help]",
    handler,
  });
}

function readBooleanEnv(name: string, fallback: boolean): boolean {
  const raw = process.env[name];
  if (raw == null) return fallback;
  const normalized = raw.trim().toLowerCase();
  if (["1", "true", "yes", "on"].includes(normalized)) return true;
  if (["0", "false", "no", "off"].includes(normalized)) return false;
  return fallback;
}

export default function omniExtension(pi: ExtensionAPI): void {
  registerOmniCommand(pi);

  pi.on("session_start", async (...args: unknown[]) => {
    const ctx = contextFromArgs(args);
    if (VERBOSE) {
      ctx?.ui?.setStatus?.("omni", `session_start ${statusLine(sessionIdFromContext(ctx))}`);
    }
    await runSessionStart(ctx);
  });

  pi.on("before_agent_start", async (rawEvent: unknown, rawCtx?: unknown) => {
    const ctx = contextFromArgs([rawCtx]);
    const sid = sessionIdFromContext(ctx);
    if (!isEnabled(sid)) {
      pendingSystemPromptAdditionBySession.delete(sid);
      return undefined;
    }

    const pending = pendingSystemPromptAdditionBySession.get(sid);
    if (!pending) return undefined;

    const event = rawEvent as BeforeAgentStartEvent;
    if (typeof event.systemPrompt !== "string") return undefined;

    pendingSystemPromptAdditionBySession.delete(sid);
    return { systemPrompt: `${event.systemPrompt}\n\n${pending}` };
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