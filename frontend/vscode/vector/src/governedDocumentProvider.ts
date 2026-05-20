import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import {
    loadDocumentTypes,
    scanGovernedDocuments,
    scanGovernedDocumentsInGroup,
    type GovernedDocument,
    type GovernedDocumentGroup,
} from "./documentDiscovery.js";
import { scanDashboards, type Dashboard } from "./dashboardDiscovery.js";

export {
    findGovernedWorkspaceRoot,
    loadDocumentTypes,
    scanGovernedDocuments,
    scanGovernedDocumentsInGroup,
} from "./documentDiscovery.js";
export type { GovernedDocument, DocTypeGlobalConfig } from "./documentDiscovery.js";
export { resolveCreateFormSource } from "./createFormResolver.js";
export type { ResolveResult } from "./createFormResolver.js";

export type GovernedGroupNode = {
    kind: "group";
    docType: string;
    groupKind: "status" | "category";
    value: string;
};

export type GovernedTreeNode =
    | { kind: "root"; docType: string }
    | GovernedGroupNode
    | { kind: "document"; doc: GovernedDocument; parent?: GovernedGroupNode }
    | { kind: "dashboard"; dashboard: Dashboard };

/**
 * Active filter applied to a specific document type's subtree.
 * - `all`      → show every document of that type
 * - `status`   → show only documents whose status matches `value`
 * - `category` → show only documents whose category matches `value`
 */
export type ActiveFilter =
    | { kind: "all" }
    | { kind: "status"; value: string }
    | { kind: "category"; value: string };

type DocumentTypeConfig = NonNullable<
    ReturnType<typeof loadDocumentTypes>
>["document-types"][string];

/**
 * Resolve a single governed document by type and numeric code.
 * Returns null when no match is found.
 */
export function resolveDocumentByCode(
    workspaceRoot: string,
    docType: string,
    code: string,
): GovernedDocument | null {
    const config = loadDocumentTypes(workspaceRoot);
    if (!config) {
        return null;
    }
    const typeConfig = config["document-types"][docType];
    if (!typeConfig) {
        return null;
    }
    const docs = scanGovernedDocuments(workspaceRoot, docType, typeConfig);
    return docs.find((d) => d.code === code) ?? null;
}

/**
 * Single tree data provider for all governed document types.
 *
 * Maintains per-type filter state so Search and List actions are scoped
 * to the owning document type.  Refresh reloads all types and preserves
 * valid filter state per type.
 */
export class PerTypeDocumentProvider implements vscode.TreeDataProvider<GovernedTreeNode> {
    private _onDidChangeTreeData = new vscode.EventEmitter<GovernedTreeNode | undefined | null>();

    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    private filters = new Map<string, ActiveFilter>();
    private treeView: vscode.TreeView<GovernedTreeNode> | undefined;
    private workspaceRoot: string;

    constructor(workspaceRoot: string) {
        this.workspaceRoot = workspaceRoot;
    }

    setTreeView(view: vscode.TreeView<GovernedTreeNode>): void {
        this.treeView = view;
        this.updateDescription();
        this.updateContextKeys();
    }

    getFilter(docType: string): ActiveFilter {
        return this.filters.get(docType) ?? { kind: "all" };
    }

    applyFilter(docType: string, filter: ActiveFilter): void {
        this.filters.set(docType, filter);
        this.updateDescription();
        this._onDidChangeTreeData.fire(undefined);
    }

    hasActiveFilters(): boolean {
        return [...this.filters.values()].some((f) => f.kind !== "all");
    }

    clearAllFilters(): void {
        for (const key of this.filters.keys()) {
            this.filters.set(key, { kind: "all" });
        }
        this.updateDescription();
        this.updateContextKeys();
        this._onDidChangeTreeData.fire(undefined);
    }

    focusType(docType: string): void {
        this._onDidChangeTreeData.fire(undefined);
        void docType;
    }

    createRootNode(docType: string): GovernedTreeNode {
        return { kind: "root", docType };
    }

    createGroupNode(docType: string, group: GovernedDocumentGroup): GovernedGroupNode {
        return {
            kind: "group",
            docType,
            groupKind: group.kind,
            value: group.value,
        };
    }

    createDocumentNode(doc: GovernedDocument): GovernedTreeNode {
        const parent =
            doc.status !== undefined
                ? this.createGroupNode(doc.type, { kind: "status", value: doc.status })
                : doc.category !== undefined
                  ? this.createGroupNode(doc.type, { kind: "category", value: doc.category })
                  : undefined;
        return parent === undefined ? { kind: "document", doc } : { kind: "document", doc, parent };
    }

    getRevealTargetForDocument(doc: GovernedDocument): GovernedTreeNode {
        return this.createDocumentNode(doc);
    }

    getRevealTargetForFilter(docType: string): GovernedTreeNode {
        const filter = this.getFilter(docType);
        if (filter.kind === "all") {
            return this.createRootNode(docType);
        }
        return this.createGroupNode(docType, {
            kind: filter.kind,
            value: filter.value,
        });
    }

    getParent(element: GovernedTreeNode): GovernedTreeNode | undefined {
        if (element.kind === "document") {
            return element.parent ?? { kind: "root", docType: element.doc.type };
        }
        if (element.kind === "group") {
            return { kind: "root", docType: element.docType };
        }
        return undefined;
    }

    getCategoryOptions(docType: string): string[] {
        const docDir = path.join(this.workspaceRoot, "doc", docType);
        if (!isDirectory(docDir)) {
            return [];
        }
        return fs
            .readdirSync(docDir, { withFileTypes: true })
            .filter((e) => e.isDirectory())
            .map((e) => e.name)
            .sort((left, right) => left.localeCompare(right));
    }

    refresh(): void {
        const config = loadDocumentTypes(this.workspaceRoot);
        if (!config) {
            this.filters.clear();
            this.updateContextKeys();
            this._onDidChangeTreeData.fire(undefined);
            return;
        }

        // Invalidate filters whose value no longer exists in the config.
        for (const [docType, filter] of this.filters.entries()) {
            if (filter.kind === "status") {
                const tc = config["document-types"][docType];
                const validStatuses = tc?.statuses ?? [];
                if (!validStatuses.includes(filter.value)) {
                    this.filters.set(docType, { kind: "all" });
                }
            }
            // category filters are validated implicitly: if the dir is gone
            // the scan returns an empty list, which is safe.
        }

        this.updateDescription();
        this.updateContextKeys();
        this._onDidChangeTreeData.fire(undefined);
    }

    getTreeItem(element: GovernedTreeNode): vscode.TreeItem {
        if (element.kind === "dashboard") {
            const item = new vscode.TreeItem(
                element.dashboard.label,
                vscode.TreeItemCollapsibleState.None,
            );
            item.contextValue = "dashboard";
            item.iconPath = new vscode.ThemeIcon("dashboard");
            item.command = {
                command: "vector.openDashboard",
                title: "Open Dashboard",
                arguments: [vscode.Uri.file(element.dashboard.filePath)],
            };
            return item;
        }

        if (element.kind === "root") {
            const config = loadDocumentTypes(this.workspaceRoot);
            const typeConfig = config?.["document-types"][element.docType];
            const hasCreateForm = typeConfig?.["create-document-form"] !== undefined;

            const filter = this.getFilter(element.docType);
            const label = element.docType.toUpperCase();
            const item = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.Collapsed);
            item.contextValue = hasCreateForm ? "docTypeFolder" : "docType";
            if (filter.kind !== "all") {
                item.description = `filter: ${filter.value}`;
            }
            return item;
        }

        if (element.kind === "group") {
            const item = new vscode.TreeItem(
                element.value,
                vscode.TreeItemCollapsibleState.Collapsed,
            );
            item.contextValue = element.groupKind;
            return item;
        }

        const doc = element.doc;
        const badge =
            doc.status !== undefined
                ? ` [${doc.status}]`
                : doc.category !== undefined
                  ? ` [${doc.category}]`
                  : "";
        const label = `${doc.code}: ${doc.title}${badge}`;
        const item = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.None);
        item.tooltip = `${doc.type}-${doc.code}-${doc.slug}`;
        item.resourceUri = vscode.Uri.file(doc.filePath);
        item.contextValue = "document";
        item.command = {
            command: "vector.openGovernedPreview",
            title: "Open Governed Preview",
            arguments: [item.resourceUri],
        };
        return item;
    }

    getChildren(element?: GovernedTreeNode): GovernedTreeNode[] {
        if (!element) {
            const dashboards = scanDashboards(this.workspaceRoot).map((dashboard) => ({
                kind: "dashboard" as const,
                dashboard,
            }));

            const config = loadDocumentTypes(this.workspaceRoot);
            if (!config) {
                return dashboards;
            }

            const docRoots = Object.keys(config["document-types"]).map((docType) => ({
                kind: "root" as const,
                docType,
            }));

            return [...dashboards, ...docRoots];
        }

        const config = loadDocumentTypes(this.workspaceRoot);
        if (!config) {
            return [];
        }

        if (element.kind === "root") {
            const typeConfig = config["document-types"][element.docType];
            if (!typeConfig) {
                return [];
            }

            if (typeConfig.layout === "directory") {
                return scanGovernedDocuments(this.workspaceRoot, element.docType, typeConfig).map(
                    (doc) => ({
                        kind: "document" as const,
                        doc,
                    }),
                );
            }

            return this.getGroupNodes(element.docType, typeConfig);
        }

        if (element.kind === "group") {
            const typeConfig = config["document-types"][element.docType];
            if (!typeConfig) {
                return [];
            }

            return scanGovernedDocumentsInGroup(this.workspaceRoot, element.docType, typeConfig, {
                kind: element.groupKind,
                value: element.value,
            }).map((doc) => ({
                kind: "document" as const,
                doc,
                parent: element,
            }));
        }

        return [];
    }

    private getGroupNodes(docType: string, typeConfig: DocumentTypeConfig): GovernedTreeNode[] {
        const docDir = path.join(this.workspaceRoot, "doc", docType);
        if (!isDirectory(docDir)) {
            return [];
        }

        const groups =
            typeConfig.layout === "status"
                ? this.getStatusGroupNodes(docType, docDir, typeConfig)
                : this.getCategoryGroupNodes(docType);
        return this.applyGroupFilter(docType, groups);
    }

    private getStatusGroupNodes(
        docType: string,
        docDir: string,
        typeConfig: DocumentTypeConfig,
    ): GovernedTreeNode[] {
        const statuses = typeConfig.statuses ?? [];
        return statuses
            .filter((status) => isDirectory(path.join(docDir, status)))
            .map((status) => ({
                kind: "group" as const,
                docType,
                groupKind: "status" as const,
                value: status,
            }));
    }

    private getCategoryGroupNodes(docType: string): GovernedTreeNode[] {
        return this.getCategoryOptions(docType).map((category) => ({
            kind: "group" as const,
            docType,
            groupKind: "category" as const,
            value: category,
        }));
    }

    private applyGroupFilter(docType: string, groups: GovernedTreeNode[]): GovernedTreeNode[] {
        const filter = this.getFilter(docType);
        if (filter.kind === "all") {
            return groups;
        }

        return groups.filter(
            (group) =>
                group.kind === "group" &&
                group.groupKind === filter.kind &&
                group.value === filter.value,
        );
    }

    private updateContextKeys(): void {
        const config = loadDocumentTypes(this.workspaceRoot);
        const hasCreateTypeForm = config?.["doc-type"]?.["create-document-type-form"] !== undefined;
        void vscode.commands.executeCommand(
            "setContext",
            "vector.hasCreateDocumentTypeForm",
            hasCreateTypeForm,
        );
    }

    private updateDescription(): void {
        const activeFilters = [...this.filters.entries()]
            .filter(([, f]) => f.kind !== "all")
            .map(
                ([type, f]) =>
                    `${type.toUpperCase()}: ${(f as { kind: string; value: string }).value}`,
            );
        const hasActive = activeFilters.length > 0;
        void vscode.commands.executeCommand("setContext", "vector.hasActiveFilter", hasActive);
        if (this.treeView) {
            this.treeView.description = hasActive ? activeFilters.join(", ") : "";
        }
    }
}

function isDirectory(targetPath: string): boolean {
    try {
        return fs.statSync(targetPath).isDirectory();
    } catch {
        return false;
    }
}
