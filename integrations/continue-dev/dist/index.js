"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.OmniContextProvider = void 0;
const child_process_1 = require("child_process");
const util_1 = require("util");
const execFileAsync = (0, util_1.promisify)(child_process_1.execFile);
class OmniContextProvider {
    constructor(params = {}) {
        this.omniPath = params.omniPath || this.detectOmniPath();
    }
    detectOmniPath() {
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
    async runCommand(command) {
        try {
            const sanitizedEnv = {};
            for (const [key, value] of Object.entries(process.env)) {
                if (value !== undefined) {
                    sanitizedEnv[key] = value;
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
            const { stdout } = await execFileAsync("sh", ["-c", `${command} 2>&1 | ${this.omniPath}`], {
                timeout: 30000,
                env: sanitizedEnv,
                maxBuffer: 10 * 1024 * 1024, // 10MB
            });
            return stdout;
        }
        catch (error) {
            if (error instanceof Error && "stdout" in error) {
                return error.stdout || error.message;
            }
            return error instanceof Error ? error.message : String(error);
        }
    }
    /**
     * Get distilled output as ContextItem for Continue.dev
     */
    async getContextItems(query) {
        if (!query.trim())
            return [];
        const output = await this.runCommand(query);
        if (!output.trim())
            return [];
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
    async getStats() {
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
        }
        catch {
            return [];
        }
    }
}
exports.OmniContextProvider = OmniContextProvider;
exports.default = OmniContextProvider;
//# sourceMappingURL=index.js.map