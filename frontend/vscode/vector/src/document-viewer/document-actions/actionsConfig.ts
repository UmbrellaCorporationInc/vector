import * as fs from "fs";
import * as path from "path";
import * as yaml from "js-yaml";
import {
    loadAgentsConfig,
    resolveProfile,
    type AgentsConfigLoad,
    type ResolvedAgent,
} from "./agentsConfig.js";
import { substituteVariables, findUnresolvedVariables } from "./variableSubstitution.js";
import { resolveGovernedPreviewSourceByIdentifier } from "../previewAssets.js";
import type { DiagnosticOutput } from "./agentExecutor.js";

export const ACTIONS_YAML_PATH = [".vector", "actions.yaml"];
export const ACTIONS_YAML_DISPLAY_PATH = ".vector/actions.yaml";

export interface ActionDefinition {
    profile?: string;
    prompt: string;
    input?: Record<string, string> | undefined;
}

export interface ActionDiagnostic {
    actionId: string;
    invalidField: string;
    configFile: string;
    message: string;
}

export interface ActionsYaml {
    /** Valid action configurations keyed by action ID. */
    actions: Record<string, ActionDefinition>;
    /** Action IDs present in configuration that failed validation. */
    invalidActionIds: Set<string>;
    /** Diagnostic errors for all invalid entries. */
    diagnostics: ActionDiagnostic[];
}

export type ActionsConfigLoad =
    | { ok: true; config: ActionsYaml }
    | { ok: false; missing: true }
    | { ok: false; missing: false; error: string; diagnostics?: ActionDiagnostic[] | undefined };

export type ResolveActionResult =
    | {
          type: "fallback";
          actionId: string;
      }
    | {
          type: "error";
          actionId: string;
          error: string;
          diagnostic?: ActionDiagnostic | undefined;
      }
    | {
          type: "resolved";
          actionId: string;
          definition: ActionDefinition;
          effectiveProfile: string;
          agents: ResolvedAgent[];
          instructionsDir: string;
          launchPromptTemplate?: string | undefined;
          mergedVariables: Record<string, string>;
          renderedPrompt: string;
      };

/**
 * Loads and validates `.vector/actions.yaml` from workspace root.
 * Validates configured entries independently so valid entries remain usable,
 * while invalid entries are tracked and reported with their action ID and invalid field.
 */
export function loadActionsConfig(
    workspaceRoot: string,
    output?: DiagnosticOutput,
): ActionsConfigLoad {
    const filePath = path.join(workspaceRoot, ...ACTIONS_YAML_PATH);

    if (!fs.existsSync(filePath)) {
        return { ok: false, missing: true };
    }

    let raw: string;
    try {
        raw = fs.readFileSync(filePath, "utf-8");
    } catch {
        return { ok: false, missing: false, error: `Cannot read ${ACTIONS_YAML_DISPLAY_PATH}` };
    }

    let parsed: unknown;
    try {
        parsed = yaml.load(raw);
    } catch {
        return {
            ok: false,
            missing: false,
            error: `${ACTIONS_YAML_DISPLAY_PATH}: YAML parse error`,
        };
    }

    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        return {
            ok: false,
            missing: false,
            error: `${ACTIONS_YAML_DISPLAY_PATH} must be a YAML mapping`,
        };
    }

    const root = parsed as Record<string, unknown>;

    for (const key of Object.keys(root)) {
        if (key !== "actions") {
            return {
                ok: false,
                missing: false,
                error: `${ACTIONS_YAML_DISPLAY_PATH}: unknown field '${key}'; only 'actions' is supported`,
            };
        }
    }

    if (!("actions" in root) || root.actions === null || root.actions === undefined) {
        return {
            ok: false,
            missing: false,
            error: `${ACTIONS_YAML_DISPLAY_PATH}: 'actions' is required`,
        };
    }

    if (typeof root.actions !== "object" || Array.isArray(root.actions)) {
        return {
            ok: false,
            missing: false,
            error: `${ACTIONS_YAML_DISPLAY_PATH}: 'actions' must be a mapping of action definitions`,
        };
    }

    const actionsMap = root.actions as Record<string, unknown>;
    const actions: Record<string, ActionDefinition> = {};
    const invalidActionIds = new Set<string>();
    const diagnostics: ActionDiagnostic[] = [];

    for (const [actionId, entry] of Object.entries(actionsMap)) {
        let isEntryValid = true;

        if (typeof actionId !== "string" || actionId.trim().length === 0) {
            const diag: ActionDiagnostic = {
                actionId,
                invalidField: "action-id",
                configFile: ACTIONS_YAML_DISPLAY_PATH,
                message: `${ACTIONS_YAML_DISPLAY_PATH}: action ID must be a non-empty string`,
            };
            diagnostics.push(diag);
            invalidActionIds.add(actionId);
            output?.appendLine(`Vector: ${diag.message}`);
            continue;
        }

        if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
            const diag: ActionDiagnostic = {
                actionId,
                invalidField: "entry",
                configFile: ACTIONS_YAML_DISPLAY_PATH,
                message: `${ACTIONS_YAML_DISPLAY_PATH}: action '${actionId}' must be a YAML mapping`,
            };
            diagnostics.push(diag);
            invalidActionIds.add(actionId);
            output?.appendLine(`Vector: ${diag.message}`);
            continue;
        }

        const entryMap = entry as Record<string, unknown>;

        // Reject unknown fields in action entry
        for (const field of Object.keys(entryMap)) {
            if (field !== "profile" && field !== "prompt" && field !== "input") {
                isEntryValid = false;
                const diag: ActionDiagnostic = {
                    actionId,
                    invalidField: field,
                    configFile: ACTIONS_YAML_DISPLAY_PATH,
                    message: `${ACTIONS_YAML_DISPLAY_PATH}: action '${actionId}' contains unknown field '${field}'`,
                };
                diagnostics.push(diag);
                output?.appendLine(`Vector: ${diag.message}`);
            }
        }

        // An omitted profile inherits the runtime action profile.
        if (
            Object.hasOwn(entryMap, "profile") &&
            (typeof entryMap.profile !== "string" || entryMap.profile.trim().length === 0)
        ) {
            isEntryValid = false;
            const diag: ActionDiagnostic = {
                actionId,
                invalidField: "profile",
                configFile: ACTIONS_YAML_DISPLAY_PATH,
                message: `${ACTIONS_YAML_DISPLAY_PATH}: action '${actionId}': 'profile' must be a non-empty string`,
            };
            diagnostics.push(diag);
            output?.appendLine(`Vector: ${diag.message}`);
        }

        // Required prompt
        if (entryMap.prompt === undefined || entryMap.prompt === null) {
            isEntryValid = false;
            const diag: ActionDiagnostic = {
                actionId,
                invalidField: "prompt",
                configFile: ACTIONS_YAML_DISPLAY_PATH,
                message: `${ACTIONS_YAML_DISPLAY_PATH}: action '${actionId}' is missing required field 'prompt'`,
            };
            diagnostics.push(diag);
            output?.appendLine(`Vector: ${diag.message}`);
        } else if (typeof entryMap.prompt !== "string" || entryMap.prompt.trim().length === 0) {
            isEntryValid = false;
            const diag: ActionDiagnostic = {
                actionId,
                invalidField: "prompt",
                configFile: ACTIONS_YAML_DISPLAY_PATH,
                message: `${ACTIONS_YAML_DISPLAY_PATH}: action '${actionId}': 'prompt' must be a non-empty string`,
            };
            diagnostics.push(diag);
            output?.appendLine(`Vector: ${diag.message}`);
        }

        // Optional input
        let validatedInput: Record<string, string> | undefined;
        if (entryMap.input !== undefined && entryMap.input !== null) {
            if (typeof entryMap.input !== "object" || Array.isArray(entryMap.input)) {
                isEntryValid = false;
                const diag: ActionDiagnostic = {
                    actionId,
                    invalidField: "input",
                    configFile: ACTIONS_YAML_DISPLAY_PATH,
                    message: `${ACTIONS_YAML_DISPLAY_PATH}: action '${actionId}': 'input' must be a mapping`,
                };
                diagnostics.push(diag);
                output?.appendLine(`Vector: ${diag.message}`);
            } else {
                validatedInput = {};
                for (const [k, v] of Object.entries(entryMap.input as Record<string, unknown>)) {
                    if (typeof k !== "string" || k.trim().length === 0) {
                        isEntryValid = false;
                        const diag: ActionDiagnostic = {
                            actionId,
                            invalidField: "input",
                            configFile: ACTIONS_YAML_DISPLAY_PATH,
                            message: `${ACTIONS_YAML_DISPLAY_PATH}: action '${actionId}': input keys must be non-empty strings`,
                        };
                        diagnostics.push(diag);
                        output?.appendLine(`Vector: ${diag.message}`);
                    } else if (typeof v === "object" && v !== null) {
                        isEntryValid = false;
                        const diag: ActionDiagnostic = {
                            actionId,
                            invalidField: "input",
                            configFile: ACTIONS_YAML_DISPLAY_PATH,
                            message: `${ACTIONS_YAML_DISPLAY_PATH}: action '${actionId}': input value for '${k}' must be a scalar`,
                        };
                        diagnostics.push(diag);
                        output?.appendLine(`Vector: ${diag.message}`);
                    } else if (
                        typeof v === "string" ||
                        typeof v === "number" ||
                        typeof v === "boolean"
                    ) {
                        validatedInput[k] = String(v);
                    } else if (v === null || v === undefined) {
                        validatedInput[k] = "";
                    }
                }
            }
        }

        if (isEntryValid) {
            actions[actionId] = {
                ...(typeof entryMap.profile === "string"
                    ? { profile: entryMap.profile.trim() }
                    : {}),
                prompt: (entryMap.prompt as string).trim(),
                ...(validatedInput !== undefined ? { input: validatedInput } : {}),
            };
        } else {
            invalidActionIds.add(actionId);
        }
    }

    return {
        ok: true,
        config: {
            actions,
            invalidActionIds,
            diagnostics,
        },
    };
}

/**
 * Merges variables in order:
 * 1. configured input defaults
 * 2. action or inline form values
 * 3. extension-generated values
 *
 * Each later source overrides duplicate keys from earlier sources.
 * profile and prompt are explicitly stripped and never merged into the variables.
 */
export function mergeActionVariables(
    configuredInput?: Record<string, string>,
    actionOrFormValues?: Record<string, string>,
    extensionGeneratedValues?: Record<string, string>,
): Record<string, string> {
    const runtimeValues: Record<string, string> = {};

    if (actionOrFormValues) {
        for (const [k, v] of Object.entries(actionOrFormValues)) {
            if (k !== "profile" && k !== "prompt") {
                runtimeValues[k] = v;
            }
        }
    }

    if (extensionGeneratedValues) {
        for (const [k, v] of Object.entries(extensionGeneratedValues)) {
            if (k !== "profile" && k !== "prompt") {
                runtimeValues[k] = v;
            }
        }
    }

    const merged: Record<string, string> = {};

    if (configuredInput) {
        for (const [k, v] of Object.entries(configuredInput)) {
            if (k !== "profile" && k !== "prompt") {
                merged[k] = substituteVariables(v, runtimeValues);
            }
        }
    }

    for (const [k, v] of Object.entries(runtimeValues)) {
        merged[k] = v;
    }

    return merged;
}

/**
 * Normalizes a relative document path to use forward slashes across all platforms
 * and strips any leading slash or dot-slash.
 */
export function normalizeDocPath(relPath: string): string {
    return relPath.replace(/\\/g, "/").replace(/^\.\//, "").replace(/^\//, "");
}

/**
 * Interpolates #{variable} placeholders in one single pass using `substituteVariables`.
 * Fails with an actionable error if any placeholder remains unresolved.
 * Sensitive runtime values or rendered prompt contents are NOT included in the error message.
 */
export function interpolateActionPrompt(
    promptContent: string,
    variables: Record<string, string>,
): { ok: true; rendered: string } | { ok: false; error: string; unresolved: string[] } {
    const unresolved = findUnresolvedVariables(promptContent, variables);
    if (unresolved.length > 0) {
        return {
            ok: false,
            error: `Vector: prompt has unresolved variables: ${unresolved.join(", ")}`,
            unresolved,
        };
    }

    const rendered = substituteVariables(promptContent, variables);
    return { ok: true, rendered };
}

/**
 * Resolves an action ID against .vector/actions.yaml, .vector/agents.yaml, and governed prompts.
 *
 * - Returns `type: "fallback"` when the configuration file or the action ID is absent.
 * - Returns `type: "error"` when the configured action is invalid, agents/prompts cannot be resolved,
 *   or interpolation fails.
 * - Returns `type: "resolved"` with all needed execution parameters on success.
 */
export function resolveConfiguredAction(options: {
    workspaceRoot: string;
    actionId: string;
    runtimeProfile?: string;
    actionOrFormValues?: Record<string, string>;
    extensionGeneratedValues?: Record<string, string>;
    actionsConfig?: ActionsConfigLoad;
    agentsConfig?: AgentsConfigLoad;
    promptResolver?: (workspaceRoot: string, promptId: string) => { content: string } | null;
    isAvailable?: (command: string) => boolean;
    output?: DiagnosticOutput;
}): ResolveActionResult {
    const {
        workspaceRoot,
        actionId,
        actionOrFormValues,
        extensionGeneratedValues,
        promptResolver = resolveGovernedPreviewSourceByIdentifier,
        isAvailable,
        output,
    } = options;

    const actionsLoad = options.actionsConfig ?? loadActionsConfig(workspaceRoot, output);
    if (!actionsLoad.ok) {
        if (actionsLoad.missing) {
            return { type: "fallback", actionId };
        }
        return {
            type: "error",
            actionId,
            error: actionsLoad.error,
        };
    }

    if (actionsLoad.config.invalidActionIds.has(actionId)) {
        const diag = actionsLoad.config.diagnostics.find((d) => d.actionId === actionId);
        const error =
            diag?.message ??
            `${ACTIONS_YAML_DISPLAY_PATH}: configured action '${actionId}' is invalid`;
        return {
            type: "error",
            actionId,
            error,
            diagnostic: diag,
        };
    }

    const actionDef = actionsLoad.config.actions[actionId];
    if (!actionDef) {
        return { type: "fallback", actionId };
    }

    const effectiveProfile = actionDef.profile ?? options.runtimeProfile?.trim();
    if (!effectiveProfile) {
        return {
            type: "error",
            actionId,
            error: `Vector: action '${actionId}' needs a profile in ${ACTIONS_YAML_DISPLAY_PATH} or from the runtime action`,
        };
    }

    const agentsLoad = options.agentsConfig ?? loadAgentsConfig(workspaceRoot);
    if (!agentsLoad.ok) {
        if (agentsLoad.missing) {
            return {
                type: "error",
                actionId,
                error: `Vector: .vector/agents.yaml is missing; cannot resolve profile '${effectiveProfile}'`,
            };
        }
        return {
            type: "error",
            actionId,
            error: agentsLoad.error,
        };
    }

    const agents = resolveProfile(agentsLoad.config, effectiveProfile, isAvailable);
    if (agents.length === 0) {
        return {
            type: "error",
            actionId,
            error: `Vector: profile '${effectiveProfile}' for action '${actionId}' has no agents in .vector/agents.yaml`,
        };
    }

    const promptDoc = promptResolver(workspaceRoot, actionDef.prompt);
    if (!promptDoc) {
        return {
            type: "error",
            actionId,
            error: `Vector: cannot resolve prompt document '${actionDef.prompt}' for action '${actionId}'`,
        };
    }

    const mergedVariables = mergeActionVariables(
        actionDef.input,
        actionOrFormValues,
        extensionGeneratedValues,
    );

    const interpolation = interpolateActionPrompt(promptDoc.content, mergedVariables);
    if (!interpolation.ok) {
        return {
            type: "error",
            actionId,
            error: interpolation.error,
        };
    }

    return {
        type: "resolved",
        actionId,
        definition: actionDef,
        effectiveProfile,
        agents,
        instructionsDir: agentsLoad.config.instructionsDir,
        launchPromptTemplate: agentsLoad.config.prompt,
        mergedVariables,
        renderedPrompt: interpolation.rendered,
    };
}
