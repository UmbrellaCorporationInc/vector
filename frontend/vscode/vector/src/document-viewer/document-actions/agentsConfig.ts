import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { spawnSync } from "child_process";
import * as yaml from "js-yaml";

export interface AgentDefinition {
    type: string;
    command: string;
}

export interface AgentsYaml {
    agents: Record<string, AgentDefinition>;
    profiles: Record<string, string[]>;
    /** Resolved value of `instructions-dir` from the YAML (variable already expanded). */
    instructionsDir: string;
    /** Optional root-level launch prompt template. When present, overrides the built-in default. */
    prompt?: string;
}

export interface ResolvedAgent {
    name: string;
    command: string;
    available: boolean;
}

export type AgentsConfigLoad =
    | { ok: true; config: AgentsYaml }
    | { ok: false; missing: true }
    | { ok: false; missing: false; error: string };

const AGENTS_YAML_PATH = [".vector", "agents.yaml"];
const AGENTS_YAML_DISPLAY_PATH = ".vector/agents.yaml";

/** The required placeholder that must appear exactly once per agent command. */
const INSTRUCTION_PLACEHOLDER = "<instruction>";
/** Legacy placeholder rejected by the new contract. */
const LEGACY_FILE_PLACEHOLDER = "<file>";
/** Legacy placeholder rejected by the new contract. */
const LEGACY_INSTRUCTION_ID_PLACEHOLDER = "<instruction-id>";

/** The only supported variable expression in `instructions-dir`. */
const SYSTEM_TEMP_VAR = "${system-temp}";
/** Matches any `${...}` variable expression. */
const VAR_PATTERN = /\$\{[^}]+\}/g;

/**
 * Loads and parses `.vector/agents.yaml` from the workspace root.
 *
 * Returns:
 *   { ok: true, config }         — file found and valid
 *   { ok: false, missing: true } — file does not exist (not an error)
 *   { ok: false, missing: false, error } — file exists but could not be parsed
 */
export function loadAgentsConfig(workspaceRoot: string): AgentsConfigLoad {
    const filePath = path.join(workspaceRoot, ...AGENTS_YAML_PATH);

    if (!fs.existsSync(filePath)) {
        return { ok: false, missing: true };
    }

    let raw: string;
    try {
        raw = fs.readFileSync(filePath, "utf-8");
    } catch {
        return { ok: false, missing: false, error: "Cannot read .vector/agents.yaml" };
    }

    let parsed: unknown;
    try {
        parsed = yaml.load(raw);
    } catch {
        return {
            ok: false,
            missing: false,
            error: `${AGENTS_YAML_DISPLAY_PATH}: YAML parse error`,
        };
    }

    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        return {
            ok: false,
            missing: false,
            error: `${AGENTS_YAML_DISPLAY_PATH} must be a YAML mapping`,
        };
    }

    const schemaError = validateAgentsYamlSchemaFields(parsed as Record<string, unknown>);
    if (schemaError !== null) {
        return { ok: false, missing: false, error: schemaError };
    }

    const map = parsed as Record<string, unknown>;

    const instructionsDirError = validateInstructionsDir(map["instructions-dir"]);
    if (instructionsDirError !== null) {
        return { ok: false, missing: false, error: instructionsDirError };
    }
    const instructionsDir = resolveInstructionsDir(map["instructions-dir"] as string);

    const agents = normaliseAgents(map.agents);
    if (!agents.ok) {
        return {
            ok: false,
            missing: false,
            error: agents.error,
        };
    }

    const profiles = normaliseProfiles(map.profiles);
    if (profiles === null) {
        return {
            ok: false,
            missing: false,
            error: `${AGENTS_YAML_DISPLAY_PATH}: 'profiles' must be a mapping of agent name lists`,
        };
    }

    const promptRaw = map["prompt"];
    if (promptRaw !== undefined && typeof promptRaw !== "string") {
        return {
            ok: false,
            missing: false,
            error: `${AGENTS_YAML_DISPLAY_PATH}: 'prompt' must be a string`,
        };
    }

    const config: AgentsYaml = { agents: agents.value, profiles, instructionsDir };
    if (typeof promptRaw === "string") {
        config.prompt = promptRaw;
    }
    return { ok: true, config };
}

/**
 * Resolves a profile name to a list of agents with PATH availability.
 * Returns an empty array when the profile does not exist.
 *
 * @param isAvailable - injectable predicate for testing; defaults to PATH check
 */
export function resolveProfile(
    config: AgentsYaml,
    profileName: string,
    isAvailable: (command: string) => boolean = isCommandInPath,
): ResolvedAgent[] {
    const agentNames = config.profiles[profileName];
    if (!agentNames || agentNames.length === 0) {
        return [];
    }

    return agentNames.flatMap((name) => {
        const def = config.agents[name];
        if (!def) {
            return [];
        }
        const executable = extractCommandExecutable(def.command);
        return [
            {
                name,
                command: def.command,
                available: executable !== null && isAvailable(executable),
            },
        ];
    });
}

/**
 * Resolves the `instructions-dir` value by substituting `${system-temp}` with
 * the platform-specific system temporary directory and normalising the result.
 *
 * The caller must have already validated the value with `validateInstructionsDir`.
 */
export function resolveInstructionsDir(instructionsDir: string): string {
    const suffix = instructionsDir.slice(SYSTEM_TEMP_VAR.length);
    return path.normalize(path.join(os.tmpdir(), suffix));
}

export function isCommandInPath(command: string): boolean {
    if (process.platform === "win32") {
        const result = spawnSync("where.exe", [command], { stdio: "ignore" });
        if (result.status === 0) {
            return true;
        }
        // where.exe may not be on PATH in all environments; fall back to PowerShell.
        // Pass the command as a positional argument so it is not interpolated into
        // the script string and is correctly forwarded to where.exe.
        const pwshResult = spawnSync("pwsh", ["-Command", "where.exe $args[0]", "--", command], {
            stdio: "ignore",
        });
        if (pwshResult.status === 0) {
            return true;
        }
        const powershellResult = spawnSync(
            "powershell.exe",
            ["-Command", "where.exe $args[0]", "--", command],
            { stdio: "ignore" },
        );
        return powershellResult.status === 0;
    }
    // Login shell (-l) gives the user's full PATH; command is passed as $1,
    // never interpolated into the script string.
    const result = spawnSync("sh", ["-lc", 'which "$1"', "--", command], {
        stdio: "ignore",
    });
    return result.status === 0;
}

export function extractCommandExecutable(commandTemplate: string): string | null {
    const trimmed = commandTemplate.trim();
    if (trimmed.length === 0) {
        return null;
    }

    if (trimmed.startsWith('"')) {
        const endQuote = trimmed.indexOf('"', 1);
        if (endQuote <= 1) {
            return null;
        }
        return trimmed.slice(1, endQuote);
    }

    const match = trimmed.match(/^[^\s]+/);
    return match?.[0] ?? null;
}

// ── Private helpers ───────────────────────────────────────────────────────────

/**
 * Validates the `instructions-dir` YAML field value.
 *
 * Returns null on success, or an actionable error message on failure.
 * Handles all cases: missing, non-string, empty, unsupported variable,
 * repeated variable, variable not at start, path traversal, and escape.
 */
function validateInstructionsDir(raw: unknown): string | null {
    if (raw === undefined || raw === null) {
        return `${AGENTS_YAML_DISPLAY_PATH}: 'instructions-dir' is required`;
    }

    if (typeof raw !== "string") {
        return `${AGENTS_YAML_DISPLAY_PATH}: 'instructions-dir' must be a non-empty string`;
    }

    if (raw.trim().length === 0) {
        return `${AGENTS_YAML_DISPLAY_PATH}: 'instructions-dir' must not be empty`;
    }

    const allVars = [...raw.matchAll(VAR_PATTERN)].map((m) => m[0]);
    const unsupported = allVars.filter((v) => v !== SYSTEM_TEMP_VAR);
    const systemTempCount = allVars.length - unsupported.length;

    if (unsupported.length > 0) {
        const badVar = unsupported[0] ?? "";
        return (
            `${AGENTS_YAML_DISPLAY_PATH}: 'instructions-dir' contains unsupported variable ` +
            `expression '${badVar}'; only \${system-temp} is supported`
        );
    }

    if (systemTempCount === 0) {
        return `${AGENTS_YAML_DISPLAY_PATH}: 'instructions-dir' must begin with \${system-temp}`;
    }

    if (systemTempCount > 1) {
        return `${AGENTS_YAML_DISPLAY_PATH}: 'instructions-dir' must contain \${system-temp} exactly once`;
    }

    if (!raw.startsWith(SYSTEM_TEMP_VAR)) {
        return `${AGENTS_YAML_DISPLAY_PATH}: 'instructions-dir' must begin with \${system-temp}`;
    }

    const suffix = raw.slice(SYSTEM_TEMP_VAR.length);

    // Reject path traversal.
    for (const segment of suffix.split(/[/\\]/)) {
        if (segment === "..") {
            return `${AGENTS_YAML_DISPLAY_PATH}: 'instructions-dir' must not contain path traversal (..)`;
        }
    }

    // Verify the resolved path stays within the system temporary directory.
    const tmpDir = path.normalize(os.tmpdir());
    const resolved = path.normalize(path.join(os.tmpdir(), suffix));
    if (resolved !== tmpDir && !resolved.startsWith(tmpDir + path.sep)) {
        return `${AGENTS_YAML_DISPLAY_PATH}: 'instructions-dir' resolves outside the system temporary directory`;
    }

    return null;
}

function normaliseAgents(
    raw: unknown,
): { ok: true; value: Record<string, AgentDefinition> } | { ok: false; error: string } {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
        return {
            ok: false,
            error: `${AGENTS_YAML_DISPLAY_PATH}: 'agents' must be a mapping of agent definitions`,
        };
    }
    const result: Record<string, AgentDefinition> = {};
    for (const [k, v] of Object.entries(raw as Record<string, unknown>)) {
        if (!v || typeof v !== "object" || Array.isArray(v)) {
            return {
                ok: false,
                error: `${AGENTS_YAML_DISPLAY_PATH}: 'agents' must be a mapping of agent definitions`,
            };
        }
        const entry = v as Record<string, unknown>;
        if (typeof entry.command !== "string") {
            return {
                ok: false,
                error: `${AGENTS_YAML_DISPLAY_PATH}: agent '${k}' must define a string command`,
            };
        }
        if (entry.command.trim().length === 0) {
            return {
                ok: false,
                error: `${AGENTS_YAML_DISPLAY_PATH}: agent '${k}' command must not be empty`,
            };
        }
        if (entry.command.includes(LEGACY_FILE_PLACEHOLDER)) {
            return {
                ok: false,
                error:
                    `${AGENTS_YAML_DISPLAY_PATH}: agent '${k}' command uses the obsolete ` +
                    `<file> placeholder; replace it with <instruction>`,
            };
        }
        if (entry.command.includes(LEGACY_INSTRUCTION_ID_PLACEHOLDER)) {
            return {
                ok: false,
                error:
                    `${AGENTS_YAML_DISPLAY_PATH}: agent '${k}' command uses the obsolete ` +
                    `<instruction-id> placeholder; replace it with <instruction>`,
            };
        }
        if (!entry.command.includes(INSTRUCTION_PLACEHOLDER)) {
            return {
                ok: false,
                error:
                    `${AGENTS_YAML_DISPLAY_PATH}: agent '${k}' command must include ` +
                    `the <instruction> placeholder`,
            };
        }
        result[k] = {
            type: typeof entry.type === "string" ? entry.type : "cli",
            command: entry.command,
        };
    }
    return { ok: true, value: result };
}

function normaliseProfiles(raw: unknown): Record<string, string[]> | null {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
        return null;
    }
    const result: Record<string, string[]> = {};
    for (const [k, v] of Object.entries(raw as Record<string, unknown>)) {
        if (!Array.isArray(v)) {
            return null;
        }
        result[k] = v.map((item) => String(item));
    }
    return result;
}

function validateAgentsYamlSchemaFields(root: Record<string, unknown>): string | null {
    return findInvalidSchemaField(root, []);
}

function findInvalidSchemaField(value: unknown, currentPath: string[]): string | null {
    if (!value || typeof value !== "object" || Array.isArray(value)) {
        return null;
    }

    const mapping = value as Record<string, unknown>;
    const dynamicChildren = hasDynamicChildren(currentPath);
    for (const [fieldName, child] of Object.entries(mapping)) {
        if (dynamicChildren) {
            const error = findInvalidSchemaField(child, [...currentPath, "*"]);
            if (error !== null) {
                return error;
            }
            continue;
        }

        if (!isKebabCaseIdentifier(fieldName)) {
            const fieldPath = [...currentPath, fieldName].join(".");
            return `${AGENTS_YAML_DISPLAY_PATH}: invalid YAML field '${fieldName}' at '${fieldPath}'; schema fields must be kebab-case`;
        }

        const error = findInvalidSchemaField(child, [...currentPath, fieldName]);
        if (error !== null) {
            return error;
        }
    }

    return null;
}

function hasDynamicChildren(currentPath: string[]): boolean {
    if (currentPath.length !== 1) {
        return false;
    }
    return currentPath[0] === "agents" || currentPath[0] === "profiles";
}

function isKebabCaseIdentifier(name: string): boolean {
    if (name.length === 0) {
        return false;
    }
    if (!/[a-z]/.test(name[0] ?? "")) {
        return false;
    }
    if (name.endsWith("-") || name.includes("--")) {
        return false;
    }
    return /^[a-z][a-z0-9-]*$/.test(name);
}
