/**
 * ESM custom loader that redirects `vscode` imports to the local stub.
 * Used only during `pnpm test` (plain Node, outside the VS Code extension host).
 */
import { fileURLToPath, pathToFileURL } from "url";
import * as path from "path";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export async function resolve(
    specifier: string,
    context: { parentURL?: string },
    nextResolve: (
        s: string,
        c: { parentURL?: string },
    ) => Promise<{ url: string; shortCircuit?: boolean }>,
): Promise<{ url: string; shortCircuit?: boolean }> {
    if (specifier === "vscode") {
        const stubPath = path.resolve(__dirname, "vscode-stub.js");
        return { url: pathToFileURL(stubPath).href, shortCircuit: true };
    }
    return nextResolve(specifier, context);
}
