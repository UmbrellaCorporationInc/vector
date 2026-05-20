import * as crypto from "crypto";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";

const activeTempFiles = new Set<string>();
const FILE_PLACEHOLDER = "<file>";

/**
 * Writes the resolved prompt content to a uniquely-named temp file.
 * The file path is tracked for cleanup on terminal close or extension deactivation.
 */
export function writeTempPrompt(content: string): string {
    const fileName = `vector-prompt-${crypto.randomUUID()}.txt`;
    const filePath = path.join(os.tmpdir(), fileName);
    fs.writeFileSync(filePath, content, "utf-8");
    activeTempFiles.add(filePath);
    return filePath;
}

/**
 * Writes content to a uniquely-named temp markdown file.
 * The file path is tracked for cleanup on extension deactivation.
 */
export function writeTempDocument(content: string): string {
    const fileName = `vector_temp_${crypto.randomUUID()}.md`;
    const filePath = path.join(os.tmpdir(), fileName);
    fs.writeFileSync(filePath, content, "utf-8");
    activeTempFiles.add(filePath);
    return filePath;
}

/**
 * Deletes a single temp file and removes it from the active tracking set.
 */
export function deleteTempFile(filePath: string): void {
    try {
        fs.unlinkSync(filePath);
    } catch {
        // Ignore - file may have already been deleted.
    }
    activeTempFiles.delete(filePath);
}

/**
 * Deletes all tracked temp files. Called on extension deactivation.
 */
export function cleanupAllTempFiles(): void {
    for (const filePath of activeTempFiles) {
        try {
            fs.unlinkSync(filePath);
        } catch {
            // Ignore.
        }
    }
    activeTempFiles.clear();
}

/**
 * Resolves an agent command template by replacing each `<file>` placeholder
 * with a safely quoted temp prompt file path.
 *
 * Throws when the configured command does not contain the required placeholder.
 */
export function resolveAgentCommand(commandTemplate: string, tempFilePath: string): string {
    if (!commandTemplate.includes(FILE_PLACEHOLDER)) {
        throw new Error(
            "Vector: agent command must include the <file> placeholder in .vector/agents.yaml",
        );
    }

    const quotedPath = quoteShellArgument(tempFilePath);
    return commandTemplate.replaceAll(FILE_PLACEHOLDER, quotedPath);
}

/**
 * Wraps a value in double quotes and escapes characters that could break
 * shell interpolation inside a quoted string.
 */
export function quoteShellArgument(value: string): string {
    const escaped = value.replace(/(["\\$`])/g, "\\$1");
    return `"${escaped}"`;
}

/**
 * Spawns a named VS Code terminal running the resolved agent command.
 * Registers a one-shot listener that deletes the temp file when the terminal closes.
 * Returns a disposable that unregisters the listener (called when the panel disposes).
 */
export function spawnAgentTerminal(
    command: string,
    agentName: string,
    label: string,
    tempFilePath: string,
    subscriptions: vscode.Disposable[],
): vscode.Terminal {
    const terminal = vscode.window.createTerminal({
        name: `Vector: ${agentName} - ${label}`,
    });

    terminal.show(false);
    terminal.sendText(resolveAgentCommand(command, tempFilePath), true);

    const onClose = vscode.window.onDidCloseTerminal((t) => {
        if (t === terminal) {
            deleteTempFile(tempFilePath);
            onClose.dispose();
        }
    });

    subscriptions.push(onClose);
    return terminal;
}
