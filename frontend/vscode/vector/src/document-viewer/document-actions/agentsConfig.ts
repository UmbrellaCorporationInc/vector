import * as fs from "fs";
import * as path from "path";
import { execSync } from "child_process";
import * as yaml from "js-yaml";

export interface AgentDefinition {
    type: string;
    command: string;
}

export interface AgentsYaml {
    agents: Record<string, AgentDefinition>;
    profiles: Record<string, string[]>;
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
const FILE_PLACEHOLDER = "<file>";
const AGENTS_YAML_DISPLAY_PATH = ".vector/agents.yaml";

/**
 * Loads and parses `.vector/agents.yaml` from the workspace root.
 *
 * Returns:
 *   { ok: true, config }      — file found and valid
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

    return { ok: true, config: { agents: agents.value, profiles } };
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

export function isCommandInPath(command: string): boolean {
    try {
        if (process.platform === "win32") {
            try {
                execSync(`pwsh -Command "where.exe ${command}"`, { stdio: "ignore" });
                return true;
            } catch {
                execSync(`powershell -Command "where.exe ${command}"`, { stdio: "ignore" });
                return true;
            }
        } else {
            execSync(`sh -lc "which ${command}"`, { stdio: "ignore" });
            return true;
        }
    } catch {
        return false;
    }
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
        if (!entry.command.includes(FILE_PLACEHOLDER)) {
            return {
                ok: false,
                error: `${AGENTS_YAML_DISPLAY_PATH}: agent '${k}' command must include the <file> placeholder`,
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
