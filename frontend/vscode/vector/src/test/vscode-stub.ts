/**
 * Minimal vscode API stub for unit tests running outside the extension host.
 *
 * Only the surface area exercised by the provider tests is implemented.
 * Everything else is left undefined — tests that call unimplemented APIs will
 * fail explicitly rather than silently.
 */

export const TreeItemCollapsibleState = {
    None: 0,
    Collapsed: 1,
    Expanded: 2,
} as const;
export type TreeItemCollapsibleState =
    (typeof TreeItemCollapsibleState)[keyof typeof TreeItemCollapsibleState];

export class TreeItem {
    label: string | undefined;
    collapsibleState: TreeItemCollapsibleState | undefined;
    description: string | undefined;
    tooltip: string | undefined;
    contextValue: string | undefined;
    resourceUri: Uri | undefined;
    command: { command: string; title: string; arguments?: unknown[] } | undefined;
    iconPath: ThemeIcon | Uri | { light: Uri | string; dark: Uri | string } | undefined;

    constructor(label: string, collapsibleState?: TreeItemCollapsibleState) {
        this.label = label;
        this.collapsibleState = collapsibleState;
    }
}

export class ThemeIcon {
    static readonly File = new ThemeIcon("file");
    static readonly Folder = new ThemeIcon("folder");
    id: string;
    constructor(id: string) {
        this.id = id;
    }
}

export class Uri {
    static file(path: string): Uri {
        return { fsPath: path };
    }
    static joinPath(base: Uri, ...segments: string[]): Uri {
        return {
            fsPath: [base.fsPath, ...segments].join("/"),
        };
    }
    fsPath: string = "";
    toString(): string {
        return this.fsPath;
    }
}

export class EventEmitter<T> {
    private listeners: ((e: T) => void)[] = [];

    get event(): (listener: (e: T) => void) => { dispose: () => void } {
        return (listener) => {
            this.listeners.push(listener);
            return {
                dispose: () => {
                    this.listeners = this.listeners.filter((l) => l !== listener);
                },
            };
        };
    }

    fire(): void {
        // no-op in tests
    }

    dispose(): void {
        this.listeners = [];
    }
}

export interface Terminal {
    show(preserveFocus?: boolean): void;
    sendText(text: string, addNewLine?: boolean): void;
}

export interface WebviewPanel {
    title: string;
    webview: {
        cspSource: string;
        html: string;
        asWebviewUri(uri: Uri): Uri;
        onDidReceiveMessage(listener: (message: unknown) => void): { dispose: () => void };
        postMessage(message: unknown): void;
    };
    reveal(): void;
    onDidChangeViewState(listener: () => void): { dispose: () => void };
    onDidDispose(listener: () => void): { dispose: () => void };
    dispose(): void;
}

type TerminalCloseListener = (terminal: Terminal) => void;
type WebviewMessageListener = (message: unknown) => void;

const terminalCloseListeners: TerminalCloseListener[] = [];
const createdTerminals: Array<{
    terminal: Terminal;
    name: string;
    sentText: string[];
    showCalls: boolean[];
}> = [];
const createdPanels: Array<{
    panel: WebviewPanel;
    title: string;
    messageListeners: WebviewMessageListener[];
    postedMessages: unknown[];
}> = [];
const errorMessages: string[] = [];
const warningMessages: string[] = [];

interface TextDocumentShowRecord {
    document: { uri: Uri };
    options: { preview?: boolean; selection?: unknown } | undefined;
}

const openedTextDocumentUris: Uri[] = [];
const shownTextDocuments: TextDocumentShowRecord[] = [];

function makeTerminal(name: string): Terminal {
    const record = {
        name,
        sentText: [] as string[],
        showCalls: [] as boolean[],
    };
    const terminal: Terminal = {
        show: (preserveFocus = false) => {
            record.showCalls.push(preserveFocus);
        },
        sendText: (text: string) => {
            record.sentText.push(text);
        },
    };
    createdTerminals.push({ terminal, ...record });
    return terminal;
}

export interface ExtensionContext {
    subscriptions: { dispose(): unknown }[];
    extensionUri: Uri;
}

const customEditorProviders = new Map<string, unknown>();

export const window = {
    createTreeView: () => ({
        description: "" as string,
        reveal: () => undefined,
        dispose: () => undefined,
    }),
    createWebviewPanel: (_viewType: string, title: string) => {
        const messageListeners: WebviewMessageListener[] = [];
        const postedMessages: unknown[] = [];
        const panel: WebviewPanel = {
            title: title.replace(/\\/g, "/").split("/").pop() ?? title,
            webview: {
                cspSource: "vscode-webview-resource:",
                html: "",
                asWebviewUri: (uri: Uri) => uri,
                onDidReceiveMessage: (listener: WebviewMessageListener) => {
                    messageListeners.push(listener);
                    return {
                        dispose: () => {
                            const index = messageListeners.indexOf(listener);
                            if (index >= 0) {
                                messageListeners.splice(index, 1);
                            }
                        },
                    };
                },
                postMessage: (message: unknown) => {
                    postedMessages.push(message);
                },
            },
            reveal: () => undefined,
            onDidChangeViewState: () => ({ dispose: () => undefined }),
            onDidDispose: () => ({ dispose: () => undefined }),
            dispose: () => undefined,
        };
        createdPanels.push({
            panel,
            get title() {
                return panel.title;
            },
            messageListeners,
            postedMessages,
        });
        return panel;
    },
    createTerminal: ({ name }: { name: string }) => makeTerminal(name),
    onDidCloseTerminal: (listener: TerminalCloseListener) => {
        terminalCloseListeners.push(listener);
        return {
            dispose: () => {
                const index = terminalCloseListeners.indexOf(listener);
                if (index >= 0) {
                    terminalCloseListeners.splice(index, 1);
                }
            },
        };
    },
    showErrorMessage: (message: string) => {
        errorMessages.push(message);
        return undefined;
    },
    showWarningMessage: (message: string) => {
        warningMessages.push(message);
        return undefined;
    },
    showQuickPick: () => Promise.resolve(undefined),
    showInputBox: () => Promise.resolve(undefined),
    showTextDocument: (
        document: { uri: Uri },
        options?: { preview?: boolean; selection?: unknown },
    ) => {
        shownTextDocuments.push({ document, options });
        return Promise.resolve(undefined);
    },
    registerCustomEditorProvider: (viewType: string, provider: unknown) => {
        customEditorProviders.set(viewType, provider);
        return { dispose: () => customEditorProviders.delete(viewType) };
    },
};

const contextValues = new Map<string, unknown>();
const commandHandlers = new Map<string, (...args: unknown[]) => unknown>();
const executedCommands: Array<{ command: string; args: unknown[] }> = [];

export const commands = {
    executeCommand: (command: string, ...args: unknown[]) => {
        executedCommands.push({ command, args });
        if (command === "setContext" && args.length >= 2) {
            contextValues.set(args[0] as string, args[1]);
        }

        if (command === "vscode.openWith" && args.length >= 2) {
            const uri = args[0] as Uri;
            const viewType = args[1] as string;
            const provider = customEditorProviders.get(viewType) as
                | {
                      openCustomDocument(uri: Uri): unknown;
                      resolveCustomEditor(document: unknown, panel: WebviewPanel): void;
                  }
                | undefined;
            if (provider) {
                const document = provider.openCustomDocument(uri);
                const panel = window.createWebviewPanel(viewType, uri.fsPath);
                provider.resolveCustomEditor(document, panel);
                return Promise.resolve(undefined);
            }
        }

        const handler = commandHandlers.get(command);
        if (handler) {
            return handler(...args);
        }
        return undefined;
    },
    registerCommand: (command: string, handler: (...args: unknown[]) => unknown) => {
        commandHandlers.set(command, handler);
        return {
            dispose: () => {
                commandHandlers.delete(command);
            },
        };
    },
};

export function __getExecutedCommands(): Array<{ command: string; args: unknown[] }> {
    return executedCommands;
}

export function __resetExecutedCommands(): void {
    executedCommands.length = 0;
}

export function __getContextValues(): Map<string, unknown> {
    return contextValues;
}

export function __resetContextValues(): void {
    contextValues.clear();
}

export function __getCommandHandler(
    command: string,
): ((...args: unknown[]) => unknown) | undefined {
    return commandHandlers.get(command);
}

export function __resetCommandHandlers(): void {
    commandHandlers.clear();
}

export interface WorkspaceFolder {
    readonly uri: Uri;
    readonly name: string;
    readonly index: number;
}

let _findFilesResults: Uri[] = [];
let _findFilesError: Error | null = null;

export const workspace = {
    workspaceFolders: undefined as WorkspaceFolder[] | undefined,
    onDidChangeTextDocument: () => ({ dispose: () => undefined }),
    openTextDocument: (uri: Uri) => {
        openedTextDocumentUris.push(uri);
        return Promise.resolve({ uri });
    },
    findFiles: (): Promise<Uri[]> => {
        if (_findFilesError) {
            return Promise.reject(_findFilesError);
        }
        return Promise.resolve(_findFilesResults.slice());
    },
};

export const ViewColumn = {
    Beside: 2,
} as const;

export function __getCreatedTerminals(): Array<{
    terminal: Terminal;
    name: string;
    sentText: string[];
    showCalls: boolean[];
}> {
    return createdTerminals;
}

export function __fireDidCloseTerminal(terminal: Terminal): void {
    for (const listener of [...terminalCloseListeners]) {
        listener(terminal);
    }
}

export function __resetTerminalState(): void {
    createdTerminals.length = 0;
    terminalCloseListeners.length = 0;
}

export function __getCreatedPanels(): Array<{
    panel: WebviewPanel;
    title: string;
    messageListeners: WebviewMessageListener[];
}> {
    return createdPanels;
}

export function __fireWebviewMessage(panel: WebviewPanel, message: unknown): void {
    const record = createdPanels.find((entry) => entry.panel === panel);
    if (!record) {
        return;
    }
    for (const listener of [...record.messageListeners]) {
        listener(message);
    }
}

export function __getErrorMessages(): string[] {
    return errorMessages;
}

export function __getWarningMessages(): string[] {
    return warningMessages;
}

export function __resetUiState(): void {
    createdPanels.length = 0;
    errorMessages.length = 0;
    warningMessages.length = 0;
    customEditorProviders.clear();
}

export function __setFindFilesResults(uris: Uri[]): void {
    _findFilesResults = uris;
    _findFilesError = null;
}

export function __mockFindFilesThrow(error: Error): void {
    _findFilesError = error;
}

export function __resetFindFilesResults(): void {
    _findFilesResults = [];
    _findFilesError = null;
}

export function __getPostedMessages(panel: WebviewPanel): unknown[] {
    const record = createdPanels.find((entry) => entry.panel === panel);
    return record?.postedMessages ?? [];
}

export function __getOpenedTextDocumentUris(): Uri[] {
    return openedTextDocumentUris;
}

export function __getShownTextDocuments(): TextDocumentShowRecord[] {
    return shownTextDocuments;
}

export function __resetTextDocumentState(): void {
    openedTextDocumentUris.length = 0;
    shownTextDocuments.length = 0;
}
