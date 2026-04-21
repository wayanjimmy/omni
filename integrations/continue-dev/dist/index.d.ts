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
export declare class OmniContextProvider {
    private omniPath;
    constructor(params?: OmniContextProviderParams);
    private detectOmniPath;
    /**
     * Run a command through OMNI pipe mode and return distilled output.
     * Uses execFile with shell:true for pipe support, and sanitizes env.
     */
    runCommand(command: string): Promise<string>;
    /**
     * Get distilled output as ContextItem for Continue.dev
     */
    getContextItems(query: string): Promise<ContextItem[]>;
    /**
     * Get OMNI stats as context
     */
    getStats(): Promise<ContextItem[]>;
}
export default OmniContextProvider;
//# sourceMappingURL=index.d.ts.map