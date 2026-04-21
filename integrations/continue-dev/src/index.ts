import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

/**
 * OMNI Signal Engine context provider for Continue.dev
 *
 * USAGE in ~/.continue/config.json:
 * {
 *   "contextProviders": [
 *     {
 *       "name": "omni",
 *       "params": {
 *         "omniPath": "/usr/local/bin/omni"  // optional
 *       }
 *     }
 *   ]
 * }
 */
export interface OmniContextProviderParams {
    omniPath?: string;
}

export interface ContextItem {
    name: string;
    description: string;
    content: string;
}

export class OmniContextProvider {
    private omniPath: string;

    constructor(params: OmniContextProviderParams = {}) {
        this.omniPath = params.omniPath || this.detectOmniPath();
    }

    private detectOmniPath(): string {
        const locations = [
            "omni",
            "/usr/local/bin/omni",
            `${process.env.HOME}/.cargo/bin/omni`,
        ];
        return locations[0]; // shell will resolve via PATH
    }

    /**
     * Run a command through OMNI pipe mode and return distilled output.
     * Uses execFile with shell:true for pipe support, and sanitizes env.
     */
    async runCommand(command: string): Promise<string> {
        try {
            const sanitizedEnv: Record<string, string> = {};
            for (const [key, value] of Object.entries(process.env)) {
                if (value !== undefined) {
                    sanitizedEnv[key] = value as string;
                }
            }
            // Remove sensitive env vars
            const denyList = [
                "AWS_SECRET_ACCESS_KEY",
                "GITHUB_TOKEN",
                "NPM_TOKEN",
                "DATABASE_URL",
            ];
            for (const key of denyList) {
                delete sanitizedEnv[key];
            }

            sanitizedEnv["OMNI_CMD"] = command;
            sanitizedEnv["OMNI_AGENT"] = "vscode_continue";

            const { stdout } = await execFileAsync(
                "sh",
                ["-c", `${command} 2>&1 | ${this.omniPath}`],
                {
                    timeout: 30000,
                    env: sanitizedEnv,
                    maxBuffer: 10 * 1024 * 1024, // 10MB
                },
            );
            return stdout;
        } catch (error: unknown) {
            if (error instanceof Error && "stdout" in error) {
                return (error as { stdout: string }).stdout || (error as Error).message;
            }
            return error instanceof Error ? error.message : String(error);
        }
    }

    /**
     * Get distilled output as ContextItem for Continue.dev
     */
    async getContextItems(query: string): Promise<ContextItem[]> {
        if (!query.trim()) return [];

        const output = await this.runCommand(query);
        if (!output.trim()) return [];

        return [
            {
                name: `OMNI: ${query.substring(0, 50)}`,
                description: `Distilled output of: ${query}`,
                content: output,
            },
        ];
    }

    /**
     * Get OMNI stats as context
     */
    async getStats(): Promise<ContextItem[]> {
        try {
            const { stdout } = await execFileAsync(this.omniPath, ["stats"], {
                timeout: 5000,
            });
            return [
                {
                    name: "OMNI Stats",
                    description: "Current session token savings",
                    content: stdout,
                },
            ];
        } catch {
            return [];
        }
    }
}

export default OmniContextProvider;
