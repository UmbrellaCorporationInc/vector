import * as assert from "assert";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { fileURLToPath } from "url";
import { loadDocumentTypes } from "../documentDiscovery.js";

function makeTempWorkspace(config: string): string {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-validate-fix-test-"));
    const vectorDir = path.join(dir, ".vector");
    fs.mkdirSync(vectorDir, { recursive: true });
    fs.writeFileSync(path.join(vectorDir, "document-types.yaml"), config, "utf-8");
    return dir;
}

suite("Phase D — validate-fix prompt resolution from document-types.yaml", () => {
    test("prompt-validate-fix value is read from doc-type block", () => {
        const config = `document-types: {}
doc-type:
  prompt-validate-fix: prompts-00007-validate-fix
`;
        const dir = makeTempWorkspace(config);
        try {
            const result = loadDocumentTypes(dir);
            assert.ok(result, "loadDocumentTypes must succeed");
            assert.strictEqual(
                result["doc-type"]?.["prompt-validate-fix"],
                "prompts-00007-validate-fix",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("prompt-validate-fix is undefined when absent from doc-type block", () => {
        const config = `document-types: {}
doc-type:
  prompt: prompts-00001-create-doc-type
`;
        const dir = makeTempWorkspace(config);
        try {
            const result = loadDocumentTypes(dir);
            assert.ok(result, "loadDocumentTypes must succeed");
            assert.strictEqual(
                result["doc-type"]?.["prompt-validate-fix"],
                undefined,
                "absent prompt-validate-fix must resolve to undefined, not throw",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("prompt-validate-fix is undefined when doc-type block itself is absent", () => {
        const config = `document-types: {}`;
        const dir = makeTempWorkspace(config);
        try {
            const result = loadDocumentTypes(dir);
            assert.ok(result, "loadDocumentTypes must succeed");
            assert.strictEqual(result["doc-type"]?.["prompt-validate-fix"], undefined);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("prompt-validate-fix coexists with other doc-type fields without interference", () => {
        const config = `document-types: {}
doc-type:
  template: template-00004-doc-type-template
  prompt: prompts-00001-create-doc-type
  prompt-validate-fix: prompts-00007-validate-fix
  create-document-type-form: form-00002-create-document-type
`;
        const dir = makeTempWorkspace(config);
        try {
            const result = loadDocumentTypes(dir);
            assert.ok(result, "loadDocumentTypes must succeed");
            const docType = result["doc-type"];
            assert.ok(docType, "doc-type section must be present");
            assert.strictEqual(docType.template, "template-00004-doc-type-template");
            assert.strictEqual(docType.prompt, "prompts-00001-create-doc-type");
            assert.strictEqual(docType["prompt-validate-fix"], "prompts-00007-validate-fix");
            assert.strictEqual(
                docType["create-document-type-form"],
                "form-00002-create-document-type",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});

suite("Phase D — package.json command and menu wiring for validate-fix", () => {
    const pkgRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
    const pkg = JSON.parse(fs.readFileSync(path.join(pkgRoot, "package.json"), "utf-8")) as {
        contributes: {
            commands: Array<{ command: string; title: string; icon?: string }>;
            menus: Record<string, Array<{ command: string; when: string; group: string }>>;
        };
    };

    test("vector.validateFix command is declared in package.json", () => {
        const cmd = pkg.contributes.commands.find((c) => c.command === "vector.validateFix");
        assert.ok(cmd, "vector.validateFix must be declared in package.json");
        assert.strictEqual(cmd.title, "Validate Fix");
    });

    test("vector.validateFix command has an icon", () => {
        const cmd = pkg.contributes.commands.find((c) => c.command === "vector.validateFix");
        assert.ok(cmd?.icon, "vector.validateFix must declare an icon for the title bar");
    });

    test("vector.validateFix appears in view/title for vector.governedDocuments", () => {
        const menus = pkg.contributes.menus["view/title"] ?? [];
        const entry = menus.find((m) => m.command === "vector.validateFix");
        assert.ok(entry, "view/title entry for vector.validateFix must exist");
        assert.ok(
            entry.when.includes("view == vector.governedDocuments"),
            `When clause must target vector.governedDocuments: "${entry.when}"`,
        );
    });

    test("vector.validateFix view/title entry is not gated on a specific tree item context", () => {
        const menus = pkg.contributes.menus["view/title"] ?? [];
        const entry = menus.find((m) => m.command === "vector.validateFix");
        assert.ok(entry, "view/title entry must exist");
        assert.ok(
            !entry.when.includes("viewItem"),
            `view/title validate-fix must be a global container action, not item-scoped: "${entry.when}"`,
        );
    });
});
