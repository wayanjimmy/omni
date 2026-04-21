import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

/**
 * OMNI Signal Engine plugin for OpenCode
 *
 * Provides three tools:
 * 1. omni_shell — run commands with OMNI distillation
 * 2. omni_stats — get token savings summary
 * 3. omni_retrieve — get full output from RewindStore
 */

interface OpenCodeTool {
    name: string;
    description: string;
    parameters: Record<string, any>;
    execute: (params: any) => Promise<any>;
}

function getOmniPath(): string {
    return process.env.OMNI_PATH || "omni";
}

function sanitizeEnv(): Record<string, string> {
    const env: Record<string, string> = {};
    const denyList = new Set([
        "AWS_SECRET_ACCESS_KEY",
        "GITHUB_TOKEN",
        "NPM_TOKEN",
        "DATABASE_URL",
    ]);
    for (const [key, value] of Object.entries(process.env)) {
        if (value !== undefined && !denyList.has(key)) {
            env[key] = value as string;
        }
    }
    return env;
}

export const tools: OpenCodeTool[] = [
    {
        name: "omni_shell",
        description:
            "Execute a shell command with OMNI intelligent distillation. " +
            "Reduces output by 80-90% by keeping only errors, warnings, and key signals. " +
            "Use instead of the standard 'shell' tool for better AI context efficiency.",
        parameters: {
            type: "object",
            properties: {
                command: {
                    type: "string",
                    description: "Shell command to execute",
                },
                workdir: {
                    type: "string",
                    description: "Working directory (optional)",
                },
            },
            required: ["command"],
        },
        async execute({
            command,
            workdir,
        }: {
            command: string;
            workdir?: string;
        }) {
            const omni = getOmniPath();
            const cwd = workdir || process.cwd();
            const env = sanitizeEnv();
            env["OMNI_CMD"] = command;
            env["OMNI_AGENT"] = "opencode";

            try {
                const { stdout } = await execFileAsync(
                    "sh",
                    ["-c", `${command} 2>&1 | ${omni}`],
                    {
                        cwd,
                        timeout: 60000,
                        maxBuffer: 10 * 1024 * 1024,
                        env,
                    },
                );

                return {
                    output: stdout || "",
                    agent: "opencode",
                    distilled: true,
                };
            } catch (error: unknown) {
                if (error instanceof Error && "stdout" in error) {
                    return {
                        output:
                            (error as { stdout: string }).stdout ||
                            (error as Error).message ||
                            "Command failed",
                        exit_code: 1,
                        distilled: false,
                    };
                }
                return {
                    output: error instanceof Error ? error.message : String(error),
                    exit_code: 1,
                    distilled: false,
                };
            }
        },
    },

    {
        name: "omni_stats",
        description: "Get OMNI token savings summary for current session",
        parameters: {
            type: "object",
            properties: {},
            required: [],
        },
        async execute() {
            const omni = getOmniPath();
            try {
                const { stdout } = await execFileAsync(omni, ["stats"], {
                    timeout: 5000,
                });
                return { stats: stdout };
            } catch {
                return { error: "OMNI not available or no stats yet" };
            }
        },
    },

    {
        name: "omni_retrieve",
        description:
            "Retrieve full (non-distilled) output from OMNI's RewindStore. " +
            'Use when OMNI\'s distilled output didn\'t include enough context.',
        parameters: {
            type: "object",
            properties: {
                hash: {
                    type: "string",
                    description:
                        'RewindStore hash from OMNI output (looks like: [OMNI: N lines omitted — omni_retrieve("HASH") for full output])',
                },
            },
            required: ["hash"],
        },
        async execute({ hash }: { hash: string }) {
            const omni = getOmniPath();
            try {
                const { stdout } = await execFileAsync(omni, ["rewind", hash], {
                    timeout: 5000,
                });
                return { content: stdout };
            } catch (error: unknown) {
                const msg =
                    error instanceof Error ? error.message : String(error);
                return { error: `Could not retrieve: ${msg}` };
            }
        },
    },
];

export default { tools };
