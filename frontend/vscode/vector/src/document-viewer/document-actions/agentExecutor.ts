import * as crypto from "crypto";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";

/** The placeholder for instruction directives in agent command templates. */
const INSTRUCTION_PLACEHOLDER = "<instruction>";

/** Placeholder within a launch prompt template that is replaced with the instruction UUID. */
const INSTRUCTION_ID_PLACEHOLDER_IN_PROMPT = "<instruction-id>";

/**
 * Built-in default launch prompt used when no root-level `prompt` is configured in agents.yaml.
 *
 * Contains one `<instruction-id>` token that is substituted with the UUID at launch time.
 * Does not use the word "server" so it remains accurate across transport variants.
 */
export const DEFAULT_AGENT_PROMPT = `Using Vector MCP, call get_instruction with id "${INSTRUCTION_ID_PLACEHOLDER_IN_PROMPT}" and execute the returned instructions.`;

/** Fixed prefix of the instruction filename (before the UUID). */
const INSTRUCTION_FILE_PREFIX = "vector-instruction-";

/** Fixed suffix of the instruction filename (after the UUID). */
const INSTRUCTION_FILE_SUFFIX = ".txt";

/**
 * Pattern that a filename must match to be treated as a recognized instruction
 * file eligible for stale-file cleanup on activation.
 *
 * The UUID segment is the canonical hyphenated lowercase form produced by
 * `crypto.randomUUID()`: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx.
 */
const INSTRUCTION_FILENAME_PATTERN =
    /^vector-instruction-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\.txt$/;

/** Instruction and temp-document files created this session, tracked for deactivation cleanup. */
const activeTempFiles = new Set<string>();

/**
 * Minimal sink for one-line diagnostic messages written without telemetry.
 * Satisfied by `vscode.OutputChannel` and the test stub alike.
 */
export interface DiagnosticOutput {
    appendLine(value: string): void;
}

// ── Instruction lifecycle ─────────────────────────────────────────────────────

/**
 * Resolves the launch prompt for a given UUID.
 *
 * Uses `configuredPrompt` when provided; otherwise falls back to `DEFAULT_AGENT_PROMPT`.
 * Every `<instruction-id>` occurrence in the resolved template is replaced with `uuid`.
 * Configured prompt text is preserved exactly apart from that substitution.
 */
export function resolveAgentPrompt(configuredPrompt: string | undefined, uuid: string): string {
    const template = configuredPrompt ?? DEFAULT_AGENT_PROMPT;
    return template.replaceAll(INSTRUCTION_ID_PLACEHOLDER_IN_PROMPT, uuid);
}

/**
 * Renders the built-in MCP directive for the given canonical UUID using the default prompt.
 *
 * Prefer `resolveAgentPrompt(agentsConfig.prompt, uuid)` at call sites that have access to
 * the loaded `AgentsYaml` config, so that a root-level `prompt` field from `agents.yaml` is
 * honoured. This wrapper is kept for tests and utilities that always want the built-in default.
 */
export function renderInstructionDirective(uuid: string): string {
    return resolveAgentPrompt(undefined, uuid);
}

/**
 * Creates `instructionsDir` when necessary, then exclusively creates a UTF-8
 * instruction file named `vector-instruction-<uuid>.txt` inside it.
 *
 * Applies restrictive POSIX permissions (0o600) where the platform supports it
 * (non-Windows). The file path is tracked for cleanup on extension deactivation.
 *
 * @param uuid - Canonical lowercase hyphenated UUID from `crypto.randomUUID()`.
 * @param content - Resolved prompt content written as UTF-8.
 * @param instructionsDir - Resolved absolute path to the controlled instruction directory.
 * @returns The absolute path of the created instruction file.
 * @throws When the directory cannot be created or a file with that name already exists.
 */
export function writeInstructionFile(
    uuid: string,
    content: string,
    instructionsDir: string,
): string {
    fs.mkdirSync(instructionsDir, { recursive: true });
    const fileName = `${INSTRUCTION_FILE_PREFIX}${uuid}${INSTRUCTION_FILE_SUFFIX}`;
    const filePath = path.join(instructionsDir, fileName);
    fs.writeFileSync(filePath, content, { encoding: "utf-8", flag: "wx" });
    if (process.platform !== "win32") {
        fs.chmodSync(filePath, 0o600);
    }
    activeTempFiles.add(filePath);
    return filePath;
}

/**
 * Writes content to a uniquely-named temp markdown file in the system temp
 * directory. Used for the create-document form flow (not for instruction files).
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
 * Deletes a single file and removes it from the active tracking set.
 * Used for immediate cleanup after a launch failure.
 */
export function deleteTempFile(filePath: string): void {
    try {
        fs.unlinkSync(filePath);
    } catch {
        // Ignore — file may have already been deleted.
    }
    activeTempFiles.delete(filePath);
}

/**
 * Deletes all tracked instruction and temp-document files.
 * Called on extension deactivation.
 *
 * Failures are written to `output` without telemetry. They never prevent
 * the remaining files from being attempted.
 */
export function cleanupAllTempFiles(output?: DiagnosticOutput): void {
    for (const filePath of activeTempFiles) {
        try {
            fs.unlinkSync(filePath);
        } catch (err) {
            output?.appendLine(
                `Vector: failed to delete file on deactivation: ${filePath}: ${String(err)}`,
            );
        }
    }
    activeTempFiles.clear();
}

/**
 * Removes recognized regular instruction files older than `maxAgeMs` from
 * `instructionsDir`. Called once on extension activation to clear up files
 * left by a previous session that crashed or was force-quit.
 *
 * Only files whose names match the exact `vector-instruction-<uuid>.txt` shape
 * are eligible for removal. Symbolic links, directories, and unrelated files
 * are never touched. Failures are written to `output`; they do not prevent
 * activation.
 *
 * @param instructionsDir - Resolved absolute path to the instruction directory.
 * @param maxAgeMs - Maximum age in milliseconds; defaults to 24 hours.
 * @param output - Optional diagnostic output channel for failure messages.
 * @param nowMs - Current timestamp override for testing; defaults to `Date.now()`.
 */
export function cleanupStaleInstructionFiles(
    instructionsDir: string,
    maxAgeMs: number = 24 * 60 * 60 * 1000,
    output?: DiagnosticOutput,
    nowMs: number = Date.now(),
): void {
    let entries: fs.Dirent[];
    try {
        entries = fs.readdirSync(instructionsDir, { withFileTypes: true });
    } catch {
        // Directory does not exist or is unreadable — nothing to clean.
        return;
    }

    for (const entry of entries) {
        if (!entry.isFile()) {
            // Symbolic links, directories, and other non-regular entries are skipped.
            continue;
        }
        if (!INSTRUCTION_FILENAME_PATTERN.test(entry.name)) {
            // Preserve unrelated files.
            continue;
        }
        const filePath = path.join(instructionsDir, entry.name);
        let stat: fs.Stats;
        try {
            // Recheck after stat to catch any edge cases (e.g., the Dirent was stale).
            stat = fs.statSync(filePath);
        } catch {
            continue;
        }
        if (!stat.isFile()) {
            // Reject non-plain-file targets after stat.
            continue;
        }
        const ageMs = nowMs - stat.mtimeMs;
        if (ageMs < maxAgeMs) {
            continue;
        }
        try {
            fs.unlinkSync(filePath);
        } catch (err) {
            output?.appendLine(
                `Vector: failed to remove stale instruction file: ${filePath}: ${String(err)}`,
            );
        }
    }
}

// ── Command resolution ────────────────────────────────────────────────────────

/**
 * Resolves an agent command template by replacing each `<instruction>` placeholder
 * with the provided directive string.
 *
 * The directive is substituted as-is — shell quoting is the caller's responsibility.
 * Throws when the configured command does not contain the required placeholder.
 */
export function resolveAgentCommand(commandTemplate: string, directive: string): string {
    if (!commandTemplate.includes(INSTRUCTION_PLACEHOLDER)) {
        throw new Error(
            "Vector: agent command must include the <instruction> placeholder in .vector/agents.yaml",
        );
    }

    return commandTemplate.replaceAll(INSTRUCTION_PLACEHOLDER, directive);
}

/**
 * Wraps a value in double quotes and escapes characters that could break
 * shell interpolation inside a quoted string.
 */
export function quoteShellArgument(value: string): string {
    const escaped = value.replace(/(["\\$`])/g, "\\$1");
    return `"${escaped}"`;
}

// ── Terminal launch ───────────────────────────────────────────────────────────

/**
 * Spawns a named VS Code terminal and sends the already-resolved agent command.
 *
 * The terminal is created with `workspaceRoot` as explicit `cwd` so the agent
 * inherits the correct project root regardless of the shell's default directory.
 *
 * Registers a one-shot listener that deletes `instructionFilePath` when the
 * terminal closes. Pushes the listener disposable into `subscriptions` so it is
 * disposed if the hosting panel is disposed first.
 *
 * @param resolvedCommand - The fully resolved command string (placeholders already substituted).
 * @param agentName - The agent name used to compose the terminal title.
 * @param label - The action label used to compose the terminal title.
 * @param instructionFilePath - Path of the instruction file to delete on terminal close.
 * @param subscriptions - Disposable list maintained by the hosting panel.
 * @param workspaceRoot - Absolute path to the active workspace root, set as terminal `cwd`.
 */
export function spawnAgentTerminal(
    resolvedCommand: string,
    agentName: string,
    label: string,
    instructionFilePath: string,
    subscriptions: vscode.Disposable[],
    workspaceRoot: string,
): vscode.Terminal {
    const terminal = vscode.window.createTerminal({
        name: `Vector: ${agentName} - ${label}`,
        cwd: workspaceRoot,
    });

    terminal.show(false);
    terminal.sendText(resolvedCommand, true);

    const onClose = vscode.window.onDidCloseTerminal((t) => {
        if (t === terminal) {
            deleteTempFile(instructionFilePath);
            onClose.dispose();
        }
    });

    subscriptions.push(onClose);
    return terminal;
}
