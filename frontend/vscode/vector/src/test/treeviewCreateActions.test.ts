import * as assert from "assert";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { fileURLToPath } from "url";
import * as vscode from "./vscode-stub.js";
import { PerTypeDocumentProvider } from "../governedDocumentProvider.js";

function makeTempWorkspace(config?: string): string {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-create-actions-test-"));
    if (config !== undefined) {
        const vectorDir = path.join(dir, ".vector");
        fs.mkdirSync(vectorDir, { recursive: true });
        fs.writeFileSync(path.join(vectorDir, "document-types.yaml"), config, "utf-8");
    }
    return dir;
}

suite("Phase B — Treeview Commands and Action Surfaces", () => {
    suite("Per-doc-type Create Document action visibility", () => {
        test("root node gets contextValue docTypeFolder when create-document-form is configured", () => {
            const config = `document-types:
  rfc:
    layout: status
    code-width: 5
    create-document-form: prompts-00010-rfc-create-form
`;
            const dir = makeTempWorkspace(config);
            try {
                const provider = new PerTypeDocumentProvider(dir);
                const item = provider.getTreeItem({ kind: "root", docType: "rfc" });
                assert.strictEqual(item.contextValue, "docTypeFolder");
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("root node gets contextValue docType when create-document-form is absent", () => {
            const config = `document-types:
  task:
    layout: status
    code-width: 5
`;
            const dir = makeTempWorkspace(config);
            try {
                const provider = new PerTypeDocumentProvider(dir);
                const item = provider.getTreeItem({ kind: "root", docType: "task" });
                assert.strictEqual(item.contextValue, "docType");
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("root node for unknown doc type gets contextValue docType", () => {
            const dir = makeTempWorkspace("document-types: {}");
            try {
                const provider = new PerTypeDocumentProvider(dir);
                const item = provider.getTreeItem({ kind: "root", docType: "unknown" });
                assert.strictEqual(item.contextValue, "docType");
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("group node does not get contextValue docTypeFolder", () => {
            const dir = makeTempWorkspace("document-types: {}");
            try {
                const provider = new PerTypeDocumentProvider(dir);
                const item = provider.getTreeItem({
                    kind: "group",
                    docType: "rfc",
                    groupKind: "status",
                    value: "accepted",
                });
                assert.notStrictEqual(item.contextValue, "docTypeFolder");
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("document node does not get contextValue docTypeFolder", () => {
            const dir = makeTempWorkspace("document-types: {}");
            try {
                const provider = new PerTypeDocumentProvider(dir);
                const item = provider.getTreeItem({
                    kind: "document",
                    doc: {
                        type: "rfc",
                        code: "00001",
                        slug: "some-rfc",
                        title: "Some RFC",
                        filePath: path.join(dir, "doc", "rfc", "rfc-00001-some-rfc.md"),
                    },
                });
                assert.notStrictEqual(item.contextValue, "docTypeFolder");
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });
    });

    suite("Global Create Document Type context key", () => {
        setup(() => {
            vscode.__resetContextValues();
        });

        test("setTreeView sets vector.hasCreateDocumentTypeForm to true when configured", () => {
            const config = `document-types: {}
doc-type:
  create-document-type-form: form-00002-create-document-type
`;
            const dir = makeTempWorkspace(config);
            try {
                const provider = new PerTypeDocumentProvider(dir);
                provider.setTreeView({
                    description: "",
                    reveal: () => undefined,
                    dispose: () => undefined,
                } as unknown as Parameters<typeof provider.setTreeView>[0]);
                assert.strictEqual(
                    vscode.__getContextValues().get("vector.hasCreateDocumentTypeForm"),
                    true,
                );
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("setTreeView sets vector.hasCreateDocumentTypeForm to false when not configured", () => {
            const dir = makeTempWorkspace("document-types: {}");
            try {
                const provider = new PerTypeDocumentProvider(dir);
                provider.setTreeView({
                    description: "",
                    reveal: () => undefined,
                    dispose: () => undefined,
                } as unknown as Parameters<typeof provider.setTreeView>[0]);
                assert.strictEqual(
                    vscode.__getContextValues().get("vector.hasCreateDocumentTypeForm"),
                    false,
                );
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("refresh sets vector.hasCreateDocumentTypeForm to false when not configured", () => {
            const dir = makeTempWorkspace("document-types: {}");
            try {
                const provider = new PerTypeDocumentProvider(dir);
                provider.refresh();
                assert.strictEqual(
                    vscode.__getContextValues().get("vector.hasCreateDocumentTypeForm"),
                    false,
                );
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("refresh sets vector.hasCreateDocumentTypeForm to true when configured", () => {
            const config = `document-types: {}
doc-type:
  create-document-type-form: form-00002-create-document-type
`;
            const dir = makeTempWorkspace(config);
            try {
                const provider = new PerTypeDocumentProvider(dir);
                provider.refresh();
                assert.strictEqual(
                    vscode.__getContextValues().get("vector.hasCreateDocumentTypeForm"),
                    true,
                );
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });
    });

    suite("package.json command and menu wiring", () => {
        const pkgRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
        const pkg = JSON.parse(fs.readFileSync(path.join(pkgRoot, "package.json"), "utf-8")) as {
            contributes: {
                commands: Array<{ command: string; title: string; icon?: string }>;
                menus: Record<string, Array<{ command: string; when: string; group: string }>>;
            };
        };

        test("vector.createDocument command is registered", () => {
            const cmd = pkg.contributes.commands.find((c) => c.command === "vector.createDocument");
            assert.ok(cmd, "vector.createDocument must be declared in package.json");
            assert.strictEqual(cmd.title, "Create Document");
        });

        test("vector.createDocumentType command is registered", () => {
            const cmd = pkg.contributes.commands.find(
                (c) => c.command === "vector.createDocumentType",
            );
            assert.ok(cmd, "vector.createDocumentType must be declared in package.json");
            assert.strictEqual(cmd.title, "Create Document Type");
        });

        test("vector.createDocument appears as inline action on docTypeFolder items", () => {
            const menus = pkg.contributes.menus["view/item/context"] ?? [];
            const entry = menus.find((m) => m.command === "vector.createDocument");
            assert.ok(entry, "view/item/context entry for vector.createDocument must exist");
            assert.ok(
                entry.when.includes("viewItem == docTypeFolder"),
                `When clause must target docTypeFolder: "${entry.when}"`,
            );
        });

        test("vector.createDocument does not appear on grouping or document nodes", () => {
            const menus = pkg.contributes.menus["view/item/context"] ?? [];
            const entries = menus.filter((m) => m.command === "vector.createDocument");
            for (const entry of entries) {
                assert.ok(
                    !entry.when.includes("viewItem == status") &&
                        !entry.when.includes("viewItem == category") &&
                        !entry.when.includes("viewItem == document"),
                    `vector.createDocument must not appear on group or document items: "${entry.when}"`,
                );
            }
        });

        test("vector.createDocumentType appears in view/title gated on vector.hasCreateDocumentTypeForm", () => {
            const menus = pkg.contributes.menus["view/title"] ?? [];
            const entry = menus.find((m) => m.command === "vector.createDocumentType");
            assert.ok(entry, "view/title entry for vector.createDocumentType must exist");
            assert.ok(
                entry.when.includes("vector.hasCreateDocumentTypeForm"),
                `When clause must check vector.hasCreateDocumentTypeForm: "${entry.when}"`,
            );
        });

        test("vector.createDocumentType is not bound to a specific tree item", () => {
            const menus = pkg.contributes.menus["view/title"] ?? [];
            const entry = menus.find((m) => m.command === "vector.createDocumentType");
            assert.ok(entry, "view/title entry must exist");
            assert.ok(
                !entry.when.includes("viewItem"),
                `view/title action must not depend on a selected tree item: "${entry.when}"`,
            );
        });
    });
});
