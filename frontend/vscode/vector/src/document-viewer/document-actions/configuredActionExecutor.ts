import * as crypto from "crypto";
import * as vscode from "vscode";
import type { ResolveActionResult } from "./actionsConfig.js";
import {
    deleteTempFile,
    resolveAgentCommand,
    resolveAgentPrompt,
    spawnAgentTerminal,
    writeInstructionFile,
} from "./agentExecutor.js";

type ResolvedAction = Extract<ResolveActionResult, { type: "resolved" }>;

/** Presents the resolved profile and starts its selected agent. */
export async function executeConfiguredAction(
    action: ResolvedAction,
    label: string,
    workspaceRoot: string,
    subscriptions: vscode.Disposable[],
): Promise<void> {
    const available = action.agents.filter((agent) => agent.available);
    if (available.length === 0) {
        void vscode.window.showErrorMessage(
            `Vector: no agents in profile '${action.effectiveProfile}' are installed`,
        );
        return;
    }

    let chosen = available[0];
    if (action.agents.length > 1) {
        const picked = await vscode.window.showQuickPick(
            action.agents.map((agent) => ({
                label: agent.name,
                description: agent.available ? agent.command : `${agent.command} (not installed)`,
                agent,
            })),
            { placeHolder: `Select agent for "${label}"` },
        );
        if (!picked) {
            return;
        }
        if (!picked.agent.available) {
            void vscode.window.showErrorMessage(
                `Vector: agent '${picked.agent.name}' is not available (not found in PATH)`,
            );
            return;
        }
        chosen = picked.agent;
    }
    if (!chosen) {
        return;
    }

    const uuid = crypto.randomUUID();
    let instructionFilePath: string | undefined;
    try {
        instructionFilePath = writeInstructionFile(
            uuid,
            action.renderedPrompt,
            action.instructionsDir,
        );
        const directive = resolveAgentPrompt(action.launchPromptTemplate, uuid);
        const command = resolveAgentCommand(chosen.command, directive);
        spawnAgentTerminal(
            command,
            chosen.name,
            label,
            instructionFilePath,
            subscriptions,
            workspaceRoot,
        );
    } catch (error) {
        if (instructionFilePath) {
            deleteTempFile(instructionFilePath);
        }
        const message = error instanceof Error ? error.message : String(error);
        void vscode.window.showErrorMessage(message);
    }
}
