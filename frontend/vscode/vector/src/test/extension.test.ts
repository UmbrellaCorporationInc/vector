import * as assert from "assert";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { fileURLToPath } from "url";
import * as vscode from "./vscode-stub.js";
import {
    findGovernedWorkspaceRoot,
    loadDocumentTypes,
    scanGovernedDocuments,
    scanGovernedDocumentsInGroup,
} from "../documentDiscovery.js";
import { PerTypeDocumentProvider, resolveDocumentByCode } from "../governedDocumentProvider.js";
import { GovernedDocumentEditorProvider } from "../document-viewer/index.js";
import {
    readGovernedDocumentContent,
    resolveGovernedPreviewSource,
    buildPreviewHtml,
    createGovernedMarkdownIt,
    applyGovernedRendererRules,
    renderGovernedMarkdown,
    extractHeadingOutline,
    slugifyHeadingText,
    parseGovernedStem,
    governedWikilinkPreviewPlugin,
    isWikilinkMessage,
    WIKILINK_MESSAGE_TYPE,
    WIKILINK_CLICK_SCRIPT,
    splitFrontmatter,
    renderFrontmatterPanel,
    buildFmLinkAnchor,
    changeGovernedDocumentStatus,
    readFrontmatterScalar,
    replaceFrontmatterScalar,
    isFmLinkMessage,
    FM_LINK_MESSAGE_TYPE,
    parseFormBlock,
    renderFormBlock,
    substituteVariables,
    findUnresolvedVariables,
    parseOpenDocBlock,
    isOpenDocParseError,
    renderOpenDocBlock,
    parseAgentBlock,
    isAgentBlockParseError,
    renderAgentBlock,
    loadAgentsConfig,
    resolveProfile,
    extractCommandExecutable,
    resolveAgentCommand,
    quoteShellArgument,
    spawnAgentTerminal,
    resolveFileSuggestions,
} from "../document-viewer/index.js";

const TEST_WEBVIEW = {
    cspSource: "vscode-webview-resource:",
} as Parameters<typeof buildPreviewHtml>[0];

const TEST_PREVIEW_ASSETS = {
    scriptUri: "vscode-webview-resource:/media/preview.js",
    chatInputRuntimeUri: "vscode-webview-resource:/media/chat-input-runtime.js",
    styleUri: "vscode-webview-resource:/media/preview.css",
    highlightScriptUri: "vscode-webview-resource:/media/hljs.min.js",
    highlightStyleUri: "vscode-webview-resource:/media/hljs-theme.css",
    codeMirrorImportMap: {
        "@codemirror/state":
            "vscode-webview-resource:/node_modules/@codemirror/state/dist/index.js",
        "@codemirror/view": "vscode-webview-resource:/node_modules/@codemirror/view/dist/index.js",
        "@codemirror/autocomplete":
            "vscode-webview-resource:/node_modules/@codemirror/autocomplete/dist/index.js",
        "@codemirror/commands":
            "vscode-webview-resource:/node_modules/@codemirror/commands/dist/index.js",
        "@codemirror/language":
            "vscode-webview-resource:/node_modules/@codemirror/language/dist/index.js",
        "@lezer/common": "vscode-webview-resource:/node_modules/@lezer/common/dist/index.js",
        "@lezer/highlight": "vscode-webview-resource:/node_modules/@lezer/highlight/dist/index.js",
        "@marijn/find-cluster-break":
            "vscode-webview-resource:/node_modules/@marijn/find-cluster-break/src/index.js",
        crelt: "vscode-webview-resource:/node_modules/crelt/index.js",
        "style-mod": "vscode-webview-resource:/node_modules/style-mod/src/style-mod.js",
        "w3c-keyname": "vscode-webview-resource:/node_modules/w3c-keyname/index.js",
    },
};

suite("Phase A — Minimal Plugin Scaffold", () => {
    const pkg_root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

    test("package.json exists and declares the expected extension fields", () => {
        const pkg_path = path.join(pkg_root, "package.json");
        assert.ok(fs.existsSync(pkg_path), "package.json must exist");

        const raw = fs.readFileSync(pkg_path, "utf-8");

        const pkg = JSON.parse(raw);

        assert.strictEqual(pkg.name, "vector", "extension name must be 'vector'");
        assert.ok(typeof pkg.version === "string", "version must be a string");
        assert.ok(pkg.engines?.vscode, "engines.vscode must be declared");
        assert.ok(typeof pkg.main === "string", "main entrypoint must be declared");
        assert.ok(
            typeof pkg.dependencies?.["js-yaml"] === "string",
            "js-yaml must be packaged as a runtime dependency",
        );
    });

    test("package.json activation is bound to Vector workspaces only", () => {
        const pkg_path = path.join(pkg_root, "package.json");
        const raw = fs.readFileSync(pkg_path, "utf-8");

        const pkg = JSON.parse(raw);

        const events: string[] = pkg.activationEvents ?? [];
        assert.ok(
            events.includes("workspaceContains:.vector/document-types.yaml"),
            "activation must trigger on workspaceContains:.vector/document-types.yaml",
        );
        assert.ok(
            !events.includes("onLanguage:markdown"),
            "must not activate on onLanguage:markdown after native preview removal",
        );
        assert.ok(!events.includes("*"), 'activation must not use the global "*" activator');
        assert.ok(
            !events.includes("onStartupFinished"),
            "activation must not use an onStartupFinished fallback",
        );
    });

    test("package script includes runtime dependencies in the VSIX", () => {
        const pkg_path = path.join(pkg_root, "package.json");
        const raw = fs.readFileSync(pkg_path, "utf-8");

        const pkg = JSON.parse(raw);

        assert.ok(
            typeof pkg.scripts?.package === "string" &&
                pkg.scripts.package.includes("vsce package"),
            "package script must invoke vsce package",
        );
        assert.ok(
            typeof pkg.scripts?.["vscode:prepublish"] === "string" &&
                pkg.scripts["vscode:prepublish"].includes("check") &&
                pkg.scripts["vscode:prepublish"].includes("compile"),
            "vscode:prepublish must run check before compile",
        );
    });

    test("src/extension.ts exports activate and deactivate", () => {
        const ext_path = path.join(pkg_root, "src", "extension.ts");
        assert.ok(fs.existsSync(ext_path), "src/extension.ts must exist");

        const src = fs.readFileSync(ext_path, "utf-8");
        assert.ok(src.includes("export function activate"), "activate must be exported");
        assert.ok(src.includes("export function deactivate"), "deactivate must be exported");
    });

    test("tsconfig.json exists with TS6-compatible configuration", () => {
        const tsconfig_path = path.join(pkg_root, "tsconfig.json");
        assert.ok(fs.existsSync(tsconfig_path), "tsconfig.json must exist");

        const raw = fs.readFileSync(tsconfig_path, "utf-8");

        const tsconfig = JSON.parse(raw);
        const opts = tsconfig.compilerOptions;

        assert.ok(opts?.outDir, "outDir must be configured");
        assert.ok(tsconfig.include?.includes("src"), "include must contain 'src'");

        assert.ok(
            opts?.module === "node16" || opts?.module === "nodenext",
            "module must be node16 or nodenext for TS6 compatibility",
        );
        assert.strictEqual(opts?.isolatedModules, true, "isolatedModules must be true");
        assert.strictEqual(
            opts?.erasableSyntaxOnly,
            true,
            "erasableSyntaxOnly must be true for TS7 forward compatibility",
        );
        assert.strictEqual(opts?.verbatimModuleSyntax, true, "verbatimModuleSyntax must be true");
    });

    test("package directory lives at frontend/vscode/vector/ per repository contract", () => {
        const normalized = pkg_root.replace(/\\/g, "/");
        assert.ok(
            normalized.endsWith("frontend/vscode/vector"),
            `package must be at frontend/vscode/vector, got: ${normalized}`,
        );
    });
});

suite("Task 00024 - Preview Toolbar, TOC, and Status Workflow", () => {
    test("extractHeadingOutline assigns deterministic unique ids for repeated headings", () => {
        const headings = extractHeadingOutline("# Intro\n## Intro\n## Intro\n");

        assert.deepStrictEqual(
            headings.map((heading) => heading.id),
            ["intro", "intro-2", "intro-3"],
        );
    });

    test("slugifyHeadingText removes accents and punctuation", () => {
        assert.strictEqual(slugifyHeadingText("T\u00edtulo: Secci\u00f3n #1"), "titulo-seccion-1");
    });

    test("renderGovernedMarkdown adds stable heading ids for TOC navigation", () => {
        const html = renderGovernedMarkdown("# Alpha\n## Beta\n");

        assert.ok(html.includes('<h1 id="alpha">'), "h1 must carry a stable id");
        assert.ok(html.includes('<h2 id="beta">'), "h2 must carry a stable id");
    });

    test("buildPreviewHtml renders toc panel and toc entries when headings are provided", () => {
        const html = buildPreviewHtml(
            TEST_WEBVIEW,
            "Task 00024",
            '<h1 id="alpha">Alpha</h1>',
            TEST_PREVIEW_ASSETS,
            undefined,
            {
                headings: [{ level: 1, text: "Alpha", id: "alpha" }],
            },
        );

        assert.ok(html.includes("data-toc-panel"), "toc panel aside must be present in the html");
        assert.ok(
            !html.includes('data-action="toggle-toc"'),
            "internal toc toggle button must not be present",
        );
        assert.ok(
            !html.includes('data-action="open-editor"'),
            "internal open-editor button must not be present",
        );
        assert.ok(
            html.includes('data-heading-id="alpha"'),
            "toc entry must target the rendered heading id",
        );
    });

    test("renderFrontmatterPanel renders a status select when status editor metadata is provided", () => {
        const html = renderFrontmatterPanel(
            { status: "todo", title: "Task 00024" },
            { current: "todo", options: ["todo", "in-progress", "done"] },
        );

        assert.ok(html.includes("data-status-select"), "status row must render an editable select");
        assert.ok(
            html.includes('<option value="todo" selected>'),
            "current status must be selected",
        );
        assert.ok(html.includes('<option value="done">'), "allowed statuses must be rendered");
    });

    test("replaceFrontmatterScalar updates a scalar frontmatter field in place", () => {
        const content = "---\nstatus: todo\ntitle: Example\n---\n# Body\n";
        const updated = replaceFrontmatterScalar(content, "status", "done");

        assert.strictEqual(readFrontmatterScalar(updated ?? "", "status"), "done");
        assert.ok(updated?.includes("title: Example"), "non-target fields must remain present");
    });

    test("changeGovernedDocumentStatus updates frontmatter and moves the file to the target status folder", () => {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-task-00024-"));
        fs.mkdirSync(path.join(dir, "doc", "task", "todo"), { recursive: true });
        fs.mkdirSync(path.join(dir, "doc", "task", "done"), { recursive: true });

        const sourcePath = path.join(
            dir,
            "doc",
            "task",
            "todo",
            "task-00024-improve-vs-code-governed-document-editor-toolbar-and-status-workflow.md",
        );
        fs.writeFileSync(
            sourcePath,
            "---\nstatus: todo\ntitle: Task 00024\n---\n# Body\n",
            "utf-8",
        );

        const result = changeGovernedDocumentStatus(
            dir,
            {
                type: "task",
                code: "00024",
                slug: "improve-vs-code-governed-document-editor-toolbar-and-status-workflow",
                title: "Task 00024",
                status: "todo",
                filePath: sourcePath,
            },
            "done",
            ["todo", "done"],
        );

        assert.strictEqual(
            result.filePath,
            path.join(
                dir,
                "doc",
                "task",
                "done",
                "task-00024-improve-vs-code-governed-document-editor-toolbar-and-status-workflow.md",
            ),
        );
        assert.strictEqual(readFrontmatterScalar(result.content, "status"), "done");
        assert.ok(!fs.existsSync(sourcePath), "source file must be moved away");
        assert.ok(fs.existsSync(result.filePath), "target file must exist after the move");
    });
});

suite("Phase B — Dynamic Governed View Container", () => {
    const pkg_root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

    test("package.json contributes a governed-documents view container", () => {
        const pkg_path = path.join(pkg_root, "package.json");
        const raw = fs.readFileSync(pkg_path, "utf-8");

        const pkg = JSON.parse(raw);

        const containers = pkg.contributes?.viewsContainers?.activitybar ?? [];
        const container = containers.find(
            (c: { id: string }) => c.id === "vector-governed-documents",
        );
        assert.ok(container, "must contribute a viewsContainer with id vector-governed-documents");
        assert.strictEqual(container.title, "Governed Documents", "container title mismatch");
    });

    test("package.json contributes a view inside the governed-documents container", () => {
        const pkg_path = path.join(pkg_root, "package.json");
        const raw = fs.readFileSync(pkg_path, "utf-8");

        const pkg = JSON.parse(raw);

        const views = pkg.contributes?.views?.["vector-governed-documents"] ?? [];
        const view = views.find((v: { id: string }) => v.id === "vector.governedDocuments");
        assert.ok(view, "must contribute a view with id vector.governedDocuments");
        assert.strictEqual(view.name, "Documents", "view name mismatch");
        assert.ok(
            view.when?.includes("vector.hasConfig"),
            "view must be gated by vector.hasConfig",
        );
    });

    test("package.json contributes a refresh command bound to the view title", () => {
        const pkg_path = path.join(pkg_root, "package.json");
        const raw = fs.readFileSync(pkg_path, "utf-8");

        const pkg = JSON.parse(raw);

        const commands = pkg.contributes?.commands ?? [];
        assert.ok(
            commands.some(
                (c: { command: string }) => c.command === "vector.refreshGovernedDocuments",
            ),
            "must contribute vector.refreshGovernedDocuments command",
        );

        const menus = pkg.contributes?.menus?.["view/title"] ?? [];
        assert.ok(
            menus.some(
                (m: { command: string; when: string }) =>
                    m.command === "vector.refreshGovernedDocuments" &&
                    m.when.includes("vector.governedDocuments"),
            ),
            "refresh command must be bound to view/title for vector.governedDocuments",
        );
    });

    function makeTempWorkspace(config?: string): string {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-vscode-test-"));
        if (config !== undefined) {
            const vectorDir = path.join(dir, ".vector");
            fs.mkdirSync(vectorDir, { recursive: true });
            fs.writeFileSync(path.join(vectorDir, "document-types.yaml"), config, "utf-8");
        }
        return dir;
    }

    test("loadDocumentTypes returns null when configuration is missing", () => {
        const dir = makeTempWorkspace();
        try {
            const result = loadDocumentTypes(dir);
            assert.strictEqual(
                result,
                null,
                "must return null when .vector/document-types.yaml is absent",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("findGovernedWorkspaceRoot returns the first workspace root with configuration", () => {
        const missingDir = makeTempWorkspace();
        const configuredDir = makeTempWorkspace(`doc-type:
  template: t
  prompt-template: pt
  prompt: p
document-types:
  rfc:
    layout: status
    "code-width": 5
    statuses:
      - draft
`);
        try {
            const result = findGovernedWorkspaceRoot([missingDir, configuredDir]);
            assert.strictEqual(
                result,
                configuredDir,
                "must resolve the first workspace root containing document-types.yaml",
            );
        } finally {
            fs.rmSync(missingDir, { recursive: true, force: true });
            fs.rmSync(configuredDir, { recursive: true, force: true });
        }
    });

    test("loadDocumentTypes parses a valid document-types.yaml", () => {
        const config = `doc-type:
  template: t
  prompt-template: pt
  prompt: p
document-types:
  rfc:
    layout: status
    "code-width": 5
    statuses:
      - draft
      - accepted
`;
        const dir = makeTempWorkspace(config);
        try {
            const result = loadDocumentTypes(dir);
            assert.ok(result !== null, "must return a parsed object");
            assert.ok(result["document-types"].rfc, "must contain 'rfc' document type");
            assert.strictEqual(result["document-types"].rfc.layout, "status", "layout mismatch");
            assert.deepStrictEqual(
                result["document-types"].rfc.statuses,
                ["draft", "accepted"],
                "statuses mismatch",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("loadDocumentTypes returns null for malformed YAML", () => {
        const dir = makeTempWorkspace("not: [ valid yaml: {{ bad");
        try {
            const result = loadDocumentTypes(dir);
            assert.strictEqual(result, null, "must return null for unreadable YAML");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("scanGovernedDocuments returns empty array when doc/type folder is missing", () => {
        const config = `doc-type:
  template: t
  prompt-template: pt
  prompt: p
document-types:
  rfc:
    layout: status
    "code-width": 5
    statuses:
      - draft
`;
        const dir = makeTempWorkspace(config);
        try {
            const types = loadDocumentTypes(dir);
            assert.ok(types, "document types should be loaded");
            const rfcConfig = types["document-types"].rfc;
            assert.ok(rfcConfig, "rfc config should exist");
            const docs = scanGovernedDocuments(dir, "rfc", rfcConfig);
            assert.deepStrictEqual(docs, [], "must return empty array when doc/rfc is absent");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("scanGovernedDocuments discovers status-based documents", () => {
        const config = `doc-type:
  template: t
  prompt-template: pt
  prompt: p
document-types:
  rfc:
    layout: status
    "code-width": 5
    statuses:
      - draft
      - accepted
`;
        const dir = makeTempWorkspace(config);
        try {
            const draftDir = path.join(dir, "doc", "rfc", "draft");
            fs.mkdirSync(draftDir, { recursive: true });
            fs.writeFileSync(
                path.join(draftDir, "rfc-00001-sample.md"),
                '---\ntitle: "Sample RFC"\n---\n\n# Sample\n',
                "utf-8",
            );

            const types = loadDocumentTypes(dir);
            assert.ok(types, "document types should be loaded");
            const rfcConfig = types["document-types"].rfc;
            assert.ok(rfcConfig, "rfc config should exist");
            const docs = scanGovernedDocuments(dir, "rfc", rfcConfig);
            assert.strictEqual(docs.length, 1, "must discover one document");
            const doc = docs[0];
            assert.ok(doc, "document should exist");
            assert.strictEqual(doc.code, "00001", "code mismatch");
            assert.strictEqual(doc.slug, "sample", "slug mismatch");
            assert.strictEqual(doc.title, "Sample RFC", "title must come from frontmatter");
            assert.strictEqual(doc.status, "draft", "status mismatch");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("scanGovernedDocuments discovers category-based documents", () => {
        const config = `doc-type:
  template: t
  prompt-template: pt
  prompt: p
document-types:
  spec:
    layout: category
    "code-width": 5
`;
        const dir = makeTempWorkspace(config);
        try {
            const categoryDir = path.join(dir, "doc", "spec", "api");
            fs.mkdirSync(categoryDir, { recursive: true });
            fs.writeFileSync(
                path.join(categoryDir, "spec-00002-api-contract.md"),
                '---\ntitle: "API Contract"\n---\n\n# API\n',
                "utf-8",
            );

            const types = loadDocumentTypes(dir);
            assert.ok(types, "document types should be loaded");
            const specConfig = types["document-types"].spec;
            assert.ok(specConfig, "spec config should exist");
            const docs = scanGovernedDocuments(dir, "spec", specConfig);
            assert.strictEqual(docs.length, 1, "must discover one document");
            const doc = docs[0];
            assert.ok(doc, "document should exist");
            assert.strictEqual(doc.code, "00002", "code mismatch");
            assert.strictEqual(doc.slug, "api-contract", "slug mismatch");
            assert.strictEqual(doc.title, "API Contract", "title mismatch");
            assert.strictEqual(doc.category, "api", "category mismatch");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("scanGovernedDocuments discovers directory-based documents", () => {
        const config = `doc-type:
  template: t
  prompt-template: pt
  prompt: p
document-types:
  research:
    layout: directory
    "code-width": 5
`;
        const dir = makeTempWorkspace(config);
        try {
            const researchDir = path.join(dir, "doc", "research");
            fs.mkdirSync(researchDir, { recursive: true });
            fs.writeFileSync(
                path.join(researchDir, "research-00001-flat.md"),
                '---\ntitle: "Flat Research"\n---\n\n# Flat\n',
                "utf-8",
            );

            const types = loadDocumentTypes(dir);
            assert.ok(types, "document types should be loaded");
            const researchConfig = types["document-types"].research;
            assert.ok(researchConfig, "research config should exist");
            const docs = scanGovernedDocuments(dir, "research", researchConfig);
            assert.strictEqual(docs.length, 1, "must discover one document");
            const doc = docs[0];
            assert.ok(doc, "document should exist");
            assert.strictEqual(doc.code, "00001", "code mismatch");
            assert.strictEqual(doc.slug, "flat", "slug mismatch");
            assert.strictEqual(doc.title, "Flat Research", "title mismatch");
            assert.strictEqual(doc.status, undefined, "should have no status");
            assert.strictEqual(doc.category, undefined, "should have no category");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("scanGovernedDocuments sorts by numeric code then slug", () => {
        const config = `doc-type:
  template: t
  prompt-template: pt
  prompt: p
document-types:
  task:
    layout: status
    "code-width": 5
    statuses:
      - todo
`;
        const dir = makeTempWorkspace(config);
        try {
            const statusDir = path.join(dir, "doc", "task", "todo");
            fs.mkdirSync(statusDir, { recursive: true });
            fs.writeFileSync(
                path.join(statusDir, "task-00010-beta.md"),
                "---\ntitle: Beta\n---\n",
                "utf-8",
            );
            fs.writeFileSync(
                path.join(statusDir, "task-00002-alpha.md"),
                "---\ntitle: Alpha\n---\n",
                "utf-8",
            );
            fs.writeFileSync(
                path.join(statusDir, "task-00002-zzz.md"),
                "---\ntitle: Zzz\n---\n",
                "utf-8",
            );

            const types = loadDocumentTypes(dir);
            assert.ok(types, "document types should be loaded");
            const taskConfig = types["document-types"].task;
            assert.ok(taskConfig, "task config should exist");
            const docs = scanGovernedDocuments(dir, "task", taskConfig);
            assert.strictEqual(docs.length, 3, "must discover three documents");
            assert.ok(docs[0], "first doc should exist");
            assert.ok(docs[1], "second doc should exist");
            assert.ok(docs[2], "third doc should exist");
            assert.strictEqual(docs[0].slug, "alpha", "first sort by code");
            assert.strictEqual(docs[1].slug, "zzz", "secondary sort by slug");
            assert.strictEqual(docs[2].slug, "beta", "higher code last");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});

suite("Phase C — Per-Type Tree Rendering and View Actions", () => {
    const pkg_root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

    function makeTempWorkspace(config?: string): string {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-vscode-phc-"));
        if (config !== undefined) {
            fs.mkdirSync(path.join(dir, ".vector"), { recursive: true });
            fs.writeFileSync(path.join(dir, ".vector", "document-types.yaml"), config, "utf-8");
        }
        return dir;
    }

    const STATUS_CONFIG = `document-types:
  rfc:
    layout: status
    "code-width": 5
    statuses:
      - draft
      - accepted
  task:
    layout: status
    "code-width": 5
    statuses:
      - todo
      - done
`;

    const CATEGORY_CONFIG = `document-types:
  spec:
    layout: category
    "code-width": 5
`;

    // ── package.json contributions ────────────────────────────────────────

    test("package.json contributes vector.searchInType command with search icon", () => {
        const pkg = JSON.parse(fs.readFileSync(path.join(pkg_root, "package.json"), "utf-8")) as {
            contributes: { commands: { command: string; icon?: string }[] };
        };
        const cmd = pkg.contributes.commands.find((c) => c.command === "vector.searchInType");
        assert.ok(cmd, "must contribute vector.searchInType");
        assert.ok(cmd.icon && cmd.icon.includes("search"), "searchInType must use a search icon");
    });

    test("package.json contributes vector.listByFilter command with filter icon", () => {
        const pkg = JSON.parse(fs.readFileSync(path.join(pkg_root, "package.json"), "utf-8")) as {
            contributes: { commands: { command: string; icon?: string }[] };
        };
        const cmd = pkg.contributes.commands.find((c) => c.command === "vector.listByFilter");
        assert.ok(cmd, "must contribute vector.listByFilter");
        assert.ok(cmd.icon && cmd.icon.includes("filter"), "listByFilter must use a filter icon");
    });

    test("package.json binds Search and Refresh to view/title for vector.governedDocuments", () => {
        const pkg = JSON.parse(fs.readFileSync(path.join(pkg_root, "package.json"), "utf-8")) as {
            contributes: { menus: { "view/title": { command: string; when: string }[] } };
        };
        const menus = pkg.contributes.menus["view/title"];
        assert.ok(menus, "view/title menus should exist");
        const requiredCommands = ["vector.searchInType", "vector.refreshGovernedDocuments"];
        for (const cmd of requiredCommands) {
            const entry = menus.find(
                (m) => m.command === cmd && m.when.includes("vector.governedDocuments"),
            );
            assert.ok(entry, `${cmd} must be bound to view/title for vector.governedDocuments`);
        }
    });

    test("package.json does not bind vector.listByFilter to view/title", () => {
        const pkg = JSON.parse(fs.readFileSync(path.join(pkg_root, "package.json"), "utf-8")) as {
            contributes: { menus: { "view/title": { command: string; when: string }[] } };
        };
        const menus = pkg.contributes.menus["view/title"];
        const entry = menus.find((m) => m.command === "vector.listByFilter");
        assert.strictEqual(
            entry,
            undefined,
            "vector.listByFilter must not appear in the view toolbar",
        );
    });

    // ── resolveDocumentByCode ─────────────────────────────────────────────

    test("resolveDocumentByCode returns null when no config present", () => {
        const dir = makeTempWorkspace();
        try {
            const result = resolveDocumentByCode(dir, "rfc", "00001");
            assert.strictEqual(result, null);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("resolveDocumentByCode returns null for unknown document type", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const result = resolveDocumentByCode(dir, "nonexistent", "00001");
            assert.strictEqual(result, null);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("resolveDocumentByCode finds a document by padded code", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const draftDir = path.join(dir, "doc", "rfc", "draft");
            fs.mkdirSync(draftDir, { recursive: true });
            fs.writeFileSync(
                path.join(draftDir, "rfc-00014-my-rfc.md"),
                "---\ntitle: My RFC\n---\n",
                "utf-8",
            );
            const result = resolveDocumentByCode(dir, "rfc", "00014");
            assert.ok(result, "must find the document");
            assert.strictEqual(result.code, "00014");
            assert.strictEqual(result.title, "My RFC");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("resolveDocumentByCode returns null when code does not match any document", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const draftDir = path.join(dir, "doc", "rfc", "draft");
            fs.mkdirSync(draftDir, { recursive: true });
            fs.writeFileSync(
                path.join(draftDir, "rfc-00001-sample.md"),
                "---\ntitle: S\n---\n",
                "utf-8",
            );
            const result = resolveDocumentByCode(dir, "rfc", "00099");
            assert.strictEqual(result, null);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    // ── PerTypeDocumentProvider — filter state ────────────────────────────

    test("PerTypeDocumentProvider defaults to 'all' filter for every type", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            assert.deepStrictEqual(provider.getFilter("rfc"), { kind: "all" });
            assert.deepStrictEqual(provider.getFilter("task"), { kind: "all" });
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("PerTypeDocumentProvider applyFilter scopes state to the specified type only", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("rfc", { kind: "status", value: "accepted" });

            assert.deepStrictEqual(provider.getFilter("rfc"), {
                kind: "status",
                value: "accepted",
            });
            assert.deepStrictEqual(provider.getFilter("task"), {
                kind: "all",
            });
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
    test("PerTypeDocumentProvider applyFilter stores category filters", () => {
        const dir = makeTempWorkspace(CATEGORY_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("spec", { kind: "category", value: "api" });

            assert.deepStrictEqual(provider.getFilter("spec"), {
                kind: "category",
                value: "api",
            });
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    // ── PerTypeDocumentProvider — refresh preserves valid filter ──────────

    test("PerTypeDocumentProvider refresh preserves a still-valid status filter", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("rfc", { kind: "status", value: "draft" });
            provider.refresh();
            assert.deepStrictEqual(provider.getFilter("rfc"), { kind: "status", value: "draft" });
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("PerTypeDocumentProvider refresh resets filter when status is removed from config", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("rfc", { kind: "status", value: "review" });

            // Overwrite config without "review" status
            fs.writeFileSync(
                path.join(dir, ".vector", "document-types.yaml"),
                `document-types:\n  rfc:\n    layout: status\n    "code-width": 5\n    statuses:\n      - draft\n      - accepted\n`,
                "utf-8",
            );
            provider.refresh();
            assert.deepStrictEqual(
                provider.getFilter("rfc"),
                { kind: "all" },
                "filter must be reset when status no longer exists",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("PerTypeDocumentProvider refresh resets all filters when config disappears", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("rfc", { kind: "status", value: "draft" });
            provider.applyFilter("task", { kind: "status", value: "todo" });

            // Remove config
            fs.unlinkSync(path.join(dir, ".vector", "document-types.yaml"));
            provider.refresh();

            assert.deepStrictEqual(provider.getFilter("rfc"), { kind: "all" });
            assert.deepStrictEqual(provider.getFilter("task"), { kind: "all" });
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    // ── PerTypeDocumentProvider — tree item rendering ─────────────────────

    test("getChildren returns only document-type roots on initial load", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            const roots = provider.getChildren();
            assert.strictEqual(roots.length, 2);
            assert.ok(roots.every((node) => node.kind === "root"));
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("initial root load does not inspect malformed governed document folders", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            fs.mkdirSync(path.join(dir, "doc", "rfc"), { recursive: true });
            fs.writeFileSync(path.join(dir, "doc", "rfc", "draft"), "not-a-directory", "utf-8");

            const provider = new PerTypeDocumentProvider(dir);
            const roots = provider.getChildren();
            assert.ok(
                roots.some((node) => node.kind === "root" && node.docType === "rfc"),
                "initial root load must succeed without touching doc/rfc contents",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getTreeItem root shows filter description when filter is active", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("rfc", { kind: "status", value: "draft" });
            const rfcRoot = { kind: "root" as const, docType: "rfc" };
            const item = provider.getTreeItem(rfcRoot);
            assert.ok(
                typeof item.description === "string" && item.description.includes("draft"),
                `root description must show active filter, got: ${String(item.description)}`,
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getTreeItem root has no description when filter is 'all'", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            const rfcRoot = { kind: "root" as const, docType: "rfc" };
            const item = provider.getTreeItem(rfcRoot);
            assert.ok(!item.description, "root description must be absent when filter is all");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getChildren for a status root returns only supported statuses present on disk", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            fs.mkdirSync(path.join(dir, "doc", "rfc", "draft"), { recursive: true });
            fs.mkdirSync(path.join(dir, "doc", "rfc", "accepted"), { recursive: true });
            fs.mkdirSync(path.join(dir, "doc", "rfc", "rogue"), { recursive: true });
            fs.writeFileSync(
                path.join(dir, "doc", "rfc", "draft", "rfc-00001-sample.md"),
                "this content must not be parsed for Phase B root expansion",
                "utf-8",
            );

            const provider = new PerTypeDocumentProvider(dir);
            const children = provider.getChildren({ kind: "root" as const, docType: "rfc" });
            assert.deepStrictEqual(children, [
                {
                    kind: "group" as const,
                    docType: "rfc",
                    groupKind: "status" as const,
                    value: "draft",
                },
                {
                    kind: "group" as const,
                    docType: "rfc",
                    groupKind: "status" as const,
                    value: "accepted",
                },
            ]);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getChildren for a status root returns an empty array when doc/<type>/ is missing", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            const children = provider.getChildren({ kind: "root" as const, docType: "rfc" });
            assert.deepStrictEqual(children, []);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getChildren for a category root returns discovered category directories in sorted order", () => {
        const dir = makeTempWorkspace(CATEGORY_CONFIG);
        try {
            fs.mkdirSync(path.join(dir, "doc", "spec", "zeta"), { recursive: true });
            fs.mkdirSync(path.join(dir, "doc", "spec", "api"), { recursive: true });
            fs.mkdirSync(path.join(dir, "doc", "spec", "data"), { recursive: true });
            fs.writeFileSync(path.join(dir, "doc", "spec", "README.txt"), "ignore", "utf-8");

            const provider = new PerTypeDocumentProvider(dir);
            const children = provider.getChildren({ kind: "root" as const, docType: "spec" });
            assert.deepStrictEqual(children, [
                {
                    kind: "group" as const,
                    docType: "spec",
                    groupKind: "category" as const,
                    value: "api",
                },
                {
                    kind: "group" as const,
                    docType: "spec",
                    groupKind: "category" as const,
                    value: "data",
                },
                {
                    kind: "group" as const,
                    docType: "spec",
                    groupKind: "category" as const,
                    value: "zeta",
                },
            ]);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getChildren for a directory root returns documents directly", () => {
        const config = `doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  research:
    layout: directory
    "code-width": 5
`;
        const dir = makeTempWorkspace(config);
        try {
            const researchDir = path.join(dir, "doc", "research");
            fs.mkdirSync(researchDir, { recursive: true });
            fs.writeFileSync(path.join(researchDir, "research-00001-flat.md"), "# Flat", "utf-8");

            const provider = new PerTypeDocumentProvider(dir);
            const children = provider.getChildren({ kind: "root" as const, docType: "research" });
            assert.strictEqual(children.length, 1);
            const first = children[0];
            assert.ok(first, "child should exist");
            assert.strictEqual(first.kind, "document");
            assert.strictEqual(first.doc.slug, "flat");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("scanGovernedDocumentsInGroup reads only the requested status folder", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const draftDir = path.join(dir, "doc", "rfc", "draft");
            const acceptedDir = path.join(dir, "doc", "rfc", "accepted");
            fs.mkdirSync(draftDir, { recursive: true });
            fs.mkdirSync(acceptedDir, { recursive: true });
            fs.writeFileSync(
                path.join(draftDir, "rfc-00002-beta.md"),
                "---\ntitle: Beta\n---\n",
                "utf-8",
            );
            fs.writeFileSync(
                path.join(draftDir, "rfc-00001-alpha.md"),
                "---\ntitle: Alpha\n---\n",
                "utf-8",
            );
            fs.writeFileSync(
                path.join(acceptedDir, "rfc-00003-accepted-only.md"),
                "---\ntitle: Accepted Only\n---\n",
                "utf-8",
            );

            const config = loadDocumentTypes(dir);
            assert.ok(config, "config must load");
            const rfcConfig = config["document-types"].rfc;
            assert.ok(rfcConfig, "rfc config must exist");
            const docs = scanGovernedDocumentsInGroup(dir, "rfc", rfcConfig, {
                kind: "status",
                value: "draft",
            });

            assert.deepStrictEqual(
                docs.map((doc) => ({
                    code: doc.code,
                    title: doc.title,
                    status: doc.status,
                })),
                [
                    { code: "00001", title: "Alpha", status: "draft" },
                    { code: "00002", title: "Beta", status: "draft" },
                ],
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getChildren for a status group returns only documents inside that status", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const draftDir = path.join(dir, "doc", "rfc", "draft");
            const acceptedDir = path.join(dir, "doc", "rfc", "accepted");
            fs.mkdirSync(draftDir, { recursive: true });
            fs.mkdirSync(acceptedDir, { recursive: true });
            fs.writeFileSync(
                path.join(draftDir, "rfc-00001-draft-only.md"),
                "---\ntitle: Draft Only\n---\n",
                "utf-8",
            );
            fs.writeFileSync(
                path.join(acceptedDir, "rfc-00002-accepted-only.md"),
                "---\ntitle: Accepted Only\n---\n",
                "utf-8",
            );

            const provider = new PerTypeDocumentProvider(dir);
            const draftGroup = {
                kind: "group" as const,
                docType: "rfc",
                groupKind: "status" as const,
                value: "draft",
            };
            const children = provider.getChildren(draftGroup);

            assert.strictEqual(
                children.length,
                1,
                "must return only the expanded group's documents",
            );
            const firstChild = children[0];
            assert.ok(firstChild, "first child must exist");
            assert.strictEqual(firstChild.kind, "document");
            assert.strictEqual(firstChild.doc.title, "Draft Only");
            assert.deepStrictEqual(firstChild.parent, draftGroup);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getChildren for a category group returns only documents inside that category", () => {
        const dir = makeTempWorkspace(CATEGORY_CONFIG);
        try {
            const apiDir = path.join(dir, "doc", "spec", "api");
            const dataDir = path.join(dir, "doc", "spec", "data");
            fs.mkdirSync(apiDir, { recursive: true });
            fs.mkdirSync(dataDir, { recursive: true });
            fs.writeFileSync(
                path.join(apiDir, "spec-00001-contract.md"),
                "---\ntitle: API Contract\n---\n",
                "utf-8",
            );
            fs.writeFileSync(
                path.join(dataDir, "spec-00002-schema.md"),
                "---\ntitle: Data Schema\n---\n",
                "utf-8",
            );

            const provider = new PerTypeDocumentProvider(dir);
            const apiGroup = {
                kind: "group" as const,
                docType: "spec",
                groupKind: "category" as const,
                value: "api",
            };
            const children = provider.getChildren(apiGroup);

            assert.strictEqual(
                children.length,
                1,
                "must return only the expanded group's documents",
            );
            const firstChild = children[0];
            assert.ok(firstChild, "first child must exist");
            assert.strictEqual(firstChild.kind, "document");
            assert.strictEqual(firstChild.doc.title, "API Contract");
            assert.strictEqual(firstChild.doc.category, "api");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getChildren for a root applies the active status filter to group nodes", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            fs.mkdirSync(path.join(dir, "doc", "rfc", "draft"), { recursive: true });
            fs.mkdirSync(path.join(dir, "doc", "rfc", "accepted"), { recursive: true });

            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("rfc", { kind: "status", value: "accepted" });

            const children = provider.getChildren({ kind: "root" as const, docType: "rfc" });
            assert.deepStrictEqual(children, [
                {
                    kind: "group" as const,
                    docType: "rfc",
                    groupKind: "status" as const,
                    value: "accepted",
                },
            ]);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getRevealTargetForDocument builds a document node with its group parent", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            const target = provider.getRevealTargetForDocument({
                type: "rfc",
                code: "00014",
                slug: "search-hit",
                title: "Search Hit",
                status: "draft",
                filePath: path.join(dir, "doc", "rfc", "draft", "rfc-00014-search-hit.md"),
            });

            assert.strictEqual(target.kind, "document");
            assert.deepStrictEqual(target.parent, {
                kind: "group",
                docType: "rfc",
                groupKind: "status",
                value: "draft",
            });
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getRevealTargetForFilter returns the filtered group when a filter is active", () => {
        const dir = makeTempWorkspace(CATEGORY_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("spec", { kind: "category", value: "api" });

            assert.deepStrictEqual(provider.getRevealTargetForFilter("spec"), {
                kind: "group",
                docType: "spec",
                groupKind: "category",
                value: "api",
            });
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getTreeItem renders explicit group nodes as collapsible items", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            const item = provider.getTreeItem({
                kind: "group",
                docType: "rfc",
                groupKind: "status",
                value: "draft",
            });
            assert.strictEqual(item.label, "draft");
            assert.strictEqual(item.contextValue, "status");
            assert.strictEqual(item.collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getTreeItem for document includes status badge in label", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            const item = provider.getTreeItem({
                kind: "document",
                parent: {
                    kind: "group",
                    docType: "rfc",
                    groupKind: "status",
                    value: "draft",
                },
                doc: {
                    type: "rfc",
                    code: "00001",
                    slug: "sample",
                    title: "Sample",
                    status: "draft",
                    filePath: path.join(dir, "doc", "rfc", "draft", "rfc-00001-sample.md"),
                },
            });
            const labelStr =
                typeof item.label === "string" ? item.label : JSON.stringify(item.label);
            assert.ok(
                labelStr.includes("[draft]"),
                `label must contain status badge, got: ${labelStr}`,
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getTreeItem for document includes category badge in label", () => {
        const dir = makeTempWorkspace(CATEGORY_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            const item = provider.getTreeItem({
                kind: "document",
                parent: {
                    kind: "group",
                    docType: "spec",
                    groupKind: "category",
                    value: "api",
                },
                doc: {
                    type: "spec",
                    code: "00001",
                    slug: "contract",
                    title: "Contract",
                    category: "api",
                    filePath: path.join(dir, "doc", "spec", "api", "spec-00001-contract.md"),
                },
            });
            const labelStr =
                typeof item.label === "string" ? item.label : JSON.stringify(item.label);
            assert.ok(
                labelStr.includes("[api]"),
                `label must contain category badge, got: ${labelStr}`,
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
    // ── getCategoryOptions ────────────────────────────────────────────────

    test("getCategoryOptions returns directory names from doc/<type>/", () => {
        const dir = makeTempWorkspace(CATEGORY_CONFIG);
        try {
            fs.mkdirSync(path.join(dir, "doc", "spec", "data"), { recursive: true });
            fs.mkdirSync(path.join(dir, "doc", "spec", "api"), { recursive: true });
            const provider = new PerTypeDocumentProvider(dir);
            const opts = provider.getCategoryOptions("spec");
            assert.deepStrictEqual(opts, ["api", "data"]);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getCategoryOptions returns empty array when doc/<type>/ is missing", () => {
        const dir = makeTempWorkspace(CATEGORY_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            const opts = provider.getCategoryOptions("spec");
            assert.deepStrictEqual(opts, []);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    // ── Search — short-code input ─────────────────────────────────────────

    test("resolveDocumentByCode finds a document when code has no leading zeros", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const draftDir = path.join(dir, "doc", "rfc", "draft");
            fs.mkdirSync(draftDir, { recursive: true });
            fs.writeFileSync(
                path.join(draftDir, "rfc-00014-short-code.md"),
                "---\ntitle: Short Code RFC\n---\n",
                "utf-8",
            );
            // Simulate what the Search command does: padStart before lookup.
            const code = "14".padStart(5, "0");
            const result = resolveDocumentByCode(dir, "rfc", code);
            assert.ok(result, "must find the document when code is left-padded from short input");
            assert.strictEqual(result.code, "00014");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("resolveDocumentByCode returns null for a short code that does not match any document", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const draftDir = path.join(dir, "doc", "rfc", "draft");
            fs.mkdirSync(draftDir, { recursive: true });
            fs.writeFileSync(
                path.join(draftDir, "rfc-00001-only.md"),
                "---\ntitle: Only\n---\n",
                "utf-8",
            );
            const code = "99".padStart(5, "0");
            const result = resolveDocumentByCode(dir, "rfc", code);
            assert.strictEqual(
                result,
                null,
                "must return null when no document matches the padded code",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});

suite("Phase D — Clear Filters Action", () => {
    function makeTempWorkspace(config?: string): string {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-vscode-phd-"));
        if (config !== undefined) {
            fs.mkdirSync(path.join(dir, ".vector"), { recursive: true });
            fs.writeFileSync(path.join(dir, ".vector", "document-types.yaml"), config, "utf-8");
        }
        return dir;
    }

    const STATUS_CONFIG = `document-types:
  rfc:
    layout: status
    "code-width": 5
    statuses:
      - draft
      - accepted
  task:
    layout: status
    "code-width": 5
    statuses:
      - todo
      - done
`;

    test("package.json contributes vector.clearAllFilters command with clear-all icon", () => {
        const pkg_root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
        const pkg = JSON.parse(fs.readFileSync(path.join(pkg_root, "package.json"), "utf-8")) as {
            contributes: { commands: { command: string; icon?: string }[] };
        };
        const cmd = pkg.contributes.commands.find((c) => c.command === "vector.clearAllFilters");
        assert.ok(cmd, "must contribute vector.clearAllFilters");
        assert.ok(cmd.icon?.includes("clear-all"), "clearAllFilters must use a clear-all icon");
    });

    test("package.json binds clearAllFilters to view/title with vector.hasActiveFilter when clause", () => {
        const pkg_root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
        const pkg = JSON.parse(fs.readFileSync(path.join(pkg_root, "package.json"), "utf-8")) as {
            contributes: { menus: { "view/title": { command: string; when: string }[] } };
        };
        const menus = pkg.contributes.menus["view/title"];
        const entry = menus.find(
            (m) =>
                m.command === "vector.clearAllFilters" &&
                m.when.includes("vector.governedDocuments") &&
                m.when.includes("vector.hasActiveFilter"),
        );
        assert.ok(
            entry,
            "clearAllFilters must be bound to view/title gated by vector.hasActiveFilter",
        );
    });

    test("extension.ts enables native collapse-all support for the governed tree view", () => {
        const pkg_root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        assert.ok(
            src.includes("showCollapseAll: true"),
            "governed tree view must enable the native Collapse All affordance",
        );
    });

    test("hasActiveFilters returns false when no filters are set", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            assert.strictEqual(provider.hasActiveFilters(), false);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("hasActiveFilters returns true after a non-all filter is applied", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("rfc", { kind: "status", value: "draft" });
            assert.strictEqual(provider.hasActiveFilters(), true);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("clearAllFilters resets every filter to all", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("rfc", { kind: "status", value: "draft" });
            provider.applyFilter("task", { kind: "status", value: "todo" });
            provider.clearAllFilters();
            assert.deepStrictEqual(provider.getFilter("rfc"), { kind: "all" });
            assert.deepStrictEqual(provider.getFilter("task"), { kind: "all" });
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("hasActiveFilters returns false after clearAllFilters", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("rfc", { kind: "status", value: "accepted" });
            provider.clearAllFilters();
            assert.strictEqual(provider.hasActiveFilters(), false);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("clearAllFilters on a provider with no filters applied is a no-op", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.clearAllFilters();
            assert.deepStrictEqual(provider.getFilter("rfc"), { kind: "all" });
            assert.deepStrictEqual(provider.getFilter("task"), { kind: "all" });
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    // ── auto-expand after filter ──────────────────────────────────────────

    test("applyFilter makes the filtered type's root node the correct reveal target", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const provider = new PerTypeDocumentProvider(dir);
            provider.applyFilter("task", { kind: "status", value: "todo" });

            const expectedRoot = { kind: "root" as const, docType: "task" };
            const childGroup = {
                kind: "group" as const,
                docType: "task",
                groupKind: "status" as const,
                value: "todo",
            };

            assert.deepStrictEqual(expectedRoot, provider.getParent(childGroup));
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});
suite("Phase E — Governed Preview Wiring", () => {
    const pkg_root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

    test("tree item command for documents is vector.openGovernedPreview", () => {
        const src = fs.readFileSync(
            path.join(pkg_root, "src", "governedDocumentProvider.ts"),
            "utf-8",
        );
        assert.ok(
            src.includes("vector.openGovernedPreview"),
            "document tree items must use vector.openGovernedPreview command (RFC 00015)",
        );
        assert.ok(!src.includes('"vscode.open"'), "document tree items must not use vscode.open");
    });

    test("extension.ts does not reference the native Markdown Preview bridge", () => {
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        assert.ok(!src.includes("extendMarkdownIt"), "must not expose extendMarkdownIt");
        assert.ok(!src.includes("wikilinkPlugin"), "must not import wikilinkPlugin");
        assert.ok(!src.includes("markdownApi"), "must not build a markdownApi object");
    });

    test("package.json does not contribute native Markdown Preview hooks", () => {
        const pkg = JSON.parse(fs.readFileSync(path.join(pkg_root, "package.json"), "utf-8")) as {
            contributes: Record<string, unknown>;
            activationEvents: string[];
        };
        assert.ok(
            !pkg.contributes["markdown.markdownItPlugins"],
            "must not declare markdown.markdownItPlugins",
        );
        assert.ok(
            !pkg.contributes["markdown.previewScripts"],
            "must not contribute markdown.previewScripts",
        );
        assert.ok(
            !pkg.contributes["markdown.previewStyles"],
            "must not contribute markdown.previewStyles",
        );
        assert.ok(
            !pkg.activationEvents.includes("onLanguage:markdown"),
            "must not activate on onLanguage:markdown",
        );
    });

    // ── parseGovernedStem — now owned by document-viewer ─────────────────

    test("parseGovernedStem returns type, code, slug for a valid governed stem", () => {
        const result = parseGovernedStem("rfc-00014-vs-code-governed-documents");
        assert.ok(result, "must parse a valid stem");
        assert.strictEqual(result.type, "rfc");
        assert.strictEqual(result.code, "00014");
        assert.strictEqual(result.slug, "vs-code-governed-documents");
    });

    test("parseGovernedStem returns type and code for a minimal stem", () => {
        const result = parseGovernedStem("task-00001-init");
        assert.ok(result);
        assert.strictEqual(result.type, "task");
        assert.strictEqual(result.code, "00001");
        assert.strictEqual(result.slug, "init");
    });

    test("parseGovernedStem returns null for a plain word (no type-code-slug pattern)", () => {
        assert.strictEqual(parseGovernedStem("something"), null);
    });

    test("parseGovernedStem returns null when there is no slug after the code", () => {
        assert.strictEqual(parseGovernedStem("rfc-00001"), null);
    });

    test("parseGovernedStem returns null when code is not numeric", () => {
        assert.strictEqual(parseGovernedStem("rfc-abc-something"), null);
    });

    test("parseGovernedStem trims surrounding whitespace before matching", () => {
        const result = parseGovernedStem("  rfc-00002-sample  ");
        assert.ok(result);
        assert.strictEqual(result.type, "rfc");
        assert.strictEqual(result.code, "00002");
    });
});

suite("Phase A (RFC 00015) — Governed Preview Panel Foundation", () => {
    const pkg_root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

    function makeTempWorkspace(config?: string): string {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-vscode-rfc15a-"));
        if (config !== undefined) {
            fs.mkdirSync(path.join(dir, ".vector"), { recursive: true });
            fs.writeFileSync(path.join(dir, ".vector", "document-types.yaml"), config, "utf-8");
        }
        return dir;
    }

    const STATUS_CONFIG = `document-types:
  rfc:
    layout: status
    "code-width": 5
    statuses:
      - draft
      - accepted
`;

    // ── package.json contributions ────────────────────────────────────────

    test("package.json contributes vector.openGovernedPreview command", () => {
        const pkg = JSON.parse(fs.readFileSync(path.join(pkg_root, "package.json"), "utf-8")) as {
            contributes: { commands: { command: string }[] };
        };
        const cmd = pkg.contributes.commands.find(
            (c) => c.command === "vector.openGovernedPreview",
        );
        assert.ok(cmd, "must contribute vector.openGovernedPreview command");
    });

    test("extension.ts registers vector.openGovernedPreview command", () => {
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        assert.ok(
            src.includes("vector.openGovernedPreview"),
            "extension.ts must register vector.openGovernedPreview",
        );
    });

    test("extension.ts does not use markdown.showPreview as the governed open command", () => {
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        assert.ok(
            !src.includes('"markdown.showPreview"'),
            "extension.ts must not route governed opens through markdown.showPreview",
        );
    });

    test("PerTypeDocumentProvider tree items use vector.openGovernedPreview not markdown.showPreview", () => {
        const src = fs.readFileSync(
            path.join(pkg_root, "src", "governedDocumentProvider.ts"),
            "utf-8",
        );
        assert.ok(
            src.includes("vector.openGovernedPreview"),
            "PerTypeDocumentProvider must use vector.openGovernedPreview",
        );
        assert.ok(
            !src.includes('"markdown.showPreview"'),
            "PerTypeDocumentProvider must not use markdown.showPreview",
        );
    });

    // ── readGovernedDocumentContent ───────────────────────────────────────

    test("readGovernedDocumentContent returns file text when path is valid", () => {
        const dir = makeTempWorkspace();
        try {
            const filePath = path.join(dir, "doc.md");
            fs.writeFileSync(filePath, "# Hello\n", "utf-8");
            const result = readGovernedDocumentContent(filePath);
            assert.strictEqual(result, "# Hello\n");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("readGovernedDocumentContent returns null for a missing file", () => {
        const result = readGovernedDocumentContent("/nonexistent/path/doc.md");
        assert.strictEqual(result, null, "must return null for an unreadable path");
    });

    // ── resolveGovernedPreviewSource ──────────────────────────────────────

    test("resolveGovernedPreviewSource returns null for a non-governed stem", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const result = resolveGovernedPreviewSource(dir, "NotAGovernedStem");
            assert.strictEqual(result, null);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("resolveGovernedPreviewSource returns null when document does not exist on disk", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const result = resolveGovernedPreviewSource(dir, "rfc-00099-missing");
            assert.strictEqual(result, null);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("resolveGovernedPreviewSource returns source with doc and content for a valid stem", () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            const draftDir = path.join(dir, "doc", "rfc", "draft");
            fs.mkdirSync(draftDir, { recursive: true });
            fs.writeFileSync(
                path.join(draftDir, "rfc-00001-sample.md"),
                "---\ntitle: Sample RFC\n---\n\n# Sample\n",
                "utf-8",
            );
            const result = resolveGovernedPreviewSource(dir, "rfc-00001-sample");
            assert.ok(result, "must resolve a valid stem");
            assert.strictEqual(result.doc.code, "00001");
            assert.ok(result.content.includes("# Sample"), "content must include document body");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    // ── buildPreviewHtml ──────────────────────────────────────────────────

    test("buildPreviewHtml produces a valid HTML document with the given title", () => {
        const html = buildPreviewHtml(
            TEST_WEBVIEW,
            "RFC 00001",
            "<p>body</p>",
            TEST_PREVIEW_ASSETS,
        );
        assert.ok(html.startsWith("<!DOCTYPE html>"), "must start with DOCTYPE");
        assert.ok(html.includes("<title>RFC 00001</title>"), "must include title");
        assert.ok(html.includes("<p>body</p>"), "must include body HTML");
    });

    test("buildPreviewHtml sets a Content-Security-Policy meta tag", () => {
        const html = buildPreviewHtml(TEST_WEBVIEW, "Test", "", TEST_PREVIEW_ASSETS);
        assert.ok(html.includes("Content-Security-Policy"), "must include a CSP meta tag");
    });

    test("buildPreviewHtml escapes HTML special characters in the title", () => {
        const html = buildPreviewHtml(
            TEST_WEBVIEW,
            'Title <script>alert("x")</script>',
            "",
            TEST_PREVIEW_ASSETS,
        );
        assert.ok(!html.includes("<script>"), "title must be escaped");
        assert.ok(html.includes("&lt;script&gt;"), "title special chars must be HTML-encoded");
    });
});

suite("Phase B (RFC 00015) — Base markdown-it Renderer", () => {
    // ── createGovernedMarkdownIt ──────────────────────────────────────────

    test("createGovernedMarkdownIt returns a markdown-it instance", () => {
        const md = createGovernedMarkdownIt();
        assert.ok(typeof md.render === "function", "must return an object with render()");
    });

    test("createGovernedMarkdownIt renders a paragraph to an <p> element", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("Hello world");
        assert.ok(html.includes("<p>"), "paragraph must produce a <p> element");
        assert.ok(html.includes("Hello world"), "paragraph text must appear in output");
    });

    test("createGovernedMarkdownIt decorates checked task list markers with a green x span", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("- [x] Completed item");
        assert.ok(
            html.includes('class="vector-task-marker-x"'),
            "checked task items must wrap the x in a dedicated span",
        );
        assert.ok(!html.includes("[x] Completed item"), "raw checked marker must be replaced");
        assert.ok(html.includes("Completed item"), "task content must remain visible");
    });

    test("createGovernedMarkdownIt preserves unchecked task list markers", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("- [ ] Pending item");
        assert.ok(
            html.includes('class="vector-task-marker vector-task-marker--unchecked"'),
            "unchecked task items must use the governed marker span",
        );
        assert.ok(html.includes("[ ]"), "unchecked marker text must remain visible");
    });

    test("createGovernedMarkdownIt leaves non-list checkbox text unchanged", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("[x] Standalone text");
        assert.ok(html.includes("[x] Standalone text"), "non-list checkbox text must remain plain");
        assert.ok(
            !html.includes("vector-task-marker-x"),
            "standalone checkbox text must not be decorated as a task list marker",
        );
    });

    // ── inline code renderer rule ─────────────────────────────────────────

    test("createGovernedMarkdownIt renders inline code with vector-inline-code class", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("Use `foo` here.");
        assert.ok(
            html.includes('class="vector-inline-code"'),
            "inline code must carry the vector-inline-code class",
        );
        assert.ok(html.includes(">foo<"), "inline code content must appear in output");
    });

    test("inline code renderer escapes HTML special characters", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("Use `<div>` here.");
        assert.ok(html.includes("&lt;div&gt;"), "< and > must be escaped in inline code");
        assert.ok(!html.includes("<div>"), "unescaped < must not appear in inline code output");
    });

    // ── fenced code block renderer rule ──────────────────────────────────

    test("createGovernedMarkdownIt renders fenced code blocks with vector-code-block wrapper", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("```\nconst x = 1;\n```");
        assert.ok(
            html.includes('class="vector-code-block"'),
            "fenced code block must be wrapped in vector-code-block",
        );
        assert.ok(html.includes("<pre>"), "fenced block must contain a <pre> element");
        assert.ok(html.includes("const x = 1;"), "code content must appear in output");
    });

    test("fenced code block preserves language hint in data-lang attribute", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("```typescript\nconst x = 1;\n```");
        assert.ok(
            html.includes('data-lang="typescript"'),
            "language hint must be preserved as data-lang",
        );
    });

    test("fenced code block with no language produces no data-lang attribute", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("```\nplain\n```");
        assert.ok(!html.includes("data-lang"), "must not emit data-lang when no language is given");
    });

    test("fenced code block renderer escapes HTML special characters", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("```\n<script>alert('x')</script>\n```");
        assert.ok(html.includes("&lt;script&gt;"), "< must be escaped in fenced code");
        assert.ok(!html.includes("<script>"), "unescaped <script> must not appear");
    });

    test("fenced code block with language adds language class for syntax highlighting", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("```typescript\nconst x = 1;\n```");
        assert.ok(
            html.includes('class="language-typescript"'),
            "code element must carry language-typescript class so hljs can highlight it",
        );
    });

    test("fenced code block with no language has no language class on code element", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("```\nplain block\n```");
        assert.ok(
            !html.includes('class="language-'),
            "plain block must not carry a language class — hljs must leave it unstyled",
        );
    });

    // ── table renderer rules ──────────────────────────────────────────────

    test("createGovernedMarkdownIt wraps tables in vector-table-wrap", () => {
        const md = createGovernedMarkdownIt();
        const source = "| A | B |\n|---|---|\n| 1 | 2 |";
        const html = md.render(source);
        assert.ok(
            html.includes('class="vector-table-wrap"'),
            "table must be wrapped in vector-table-wrap",
        );
    });

    test("createGovernedMarkdownIt adds vector-table class to table element", () => {
        const md = createGovernedMarkdownIt();
        const source = "| A | B |\n|---|---|\n| 1 | 2 |";
        const html = md.render(source);
        assert.ok(
            html.includes('class="vector-table"'),
            "table element must carry the vector-table class",
        );
    });

    test("table wrapper is closed correctly after the table", () => {
        const md = createGovernedMarkdownIt();
        const source = "| A | B |\n|---|---|\n| 1 | 2 |";
        const html = md.render(source);
        const wrapOpen = html.indexOf('class="vector-table-wrap"');
        const wrapClose = html.indexOf("</table></div>", wrapOpen);
        assert.ok(wrapClose > wrapOpen, "vector-table-wrap must be properly closed after </table>");
    });

    // ── applyGovernedRendererRules ────────────────────────────────────────

    test("applyGovernedRendererRules is idempotent when called twice", () => {
        const md = createGovernedMarkdownIt();
        applyGovernedRendererRules(md);
        const html = md.render("Use `foo` here.");
        assert.ok(
            html.includes('class="vector-inline-code"'),
            "rules must remain correct after double application",
        );
    });

    // ── renderGovernedMarkdown ────────────────────────────────────────────

    test("renderGovernedMarkdown returns an HTML fragment string", () => {
        const html = renderGovernedMarkdown("# Hello\n\nParagraph.");
        assert.ok(typeof html === "string", "must return a string");
        assert.ok(
            html.includes('<h1 id="hello">'),
            "heading must appear in output with a stable anchor id",
        );
        assert.ok(html.includes("<p>"), "paragraph must appear in output");
    });

    test("renderGovernedMarkdown applies inline code and table rules end-to-end", () => {
        const source = "Use `code` here.\n\n| X | Y |\n|---|---|\n| a | b |";
        const html = renderGovernedMarkdown(source);
        assert.ok(html.includes('class="vector-inline-code"'), "inline code rule must apply");
        assert.ok(html.includes('class="vector-table"'), "table rule must apply");
    });

    test("renderGovernedMarkdown does not produce raw <script> tags", () => {
        const html = renderGovernedMarkdown("<script>alert('x')</script>");
        assert.ok(!html.includes("<script>"), "raw script tags must be neutralized");
    });

    // ── parsing / presentation separation ────────────────────────────────

    test("markdownRenderer.ts exports createGovernedMarkdownIt, applyGovernedRendererRules, renderGovernedMarkdown", () => {
        assert.ok(typeof createGovernedMarkdownIt === "function");
        assert.ok(typeof applyGovernedRendererRules === "function");
        assert.ok(typeof renderGovernedMarkdown === "function");
    });
});

suite("Phase C (RFC 00015) — Governed Wikilink Parsing and Same-Panel Navigation", () => {
    // ── governedWikilinkPreviewPlugin — token transformation ─────────────

    function makeState(text: string): {
        tokens: Array<{
            type: string;
            content: string;
            children: Array<{ type: string; content: string }> | null;
        }>;
        Token: new (
            type: string,
            tag: string,
            nesting: number,
        ) => { type: string; content: string };
    } {
        class Token {
            type: string;
            content: string;
            constructor(type: string) {
                this.type = type;
                this.content = "";
            }
        }
        const child = new Token("text");
        child.content = text;
        return {
            tokens: [{ type: "inline", content: "", children: [child] }],
            Token,
        };
    }

    interface StubMd {
        core: {
            ruler: { push(name: string, fn: (s: ReturnType<typeof makeState>) => void): void };
        };
        use(plugin: (md: StubMd) => void): StubMd;
    }

    function applyPreviewPlugin(text: string): Array<{ type: string; content: string }> {
        const state = makeState(text);
        const md: StubMd = {
            core: {
                ruler: {
                    push(_name: string, fn: (s: ReturnType<typeof makeState>) => void) {
                        fn(state);
                    },
                },
            },
            use(plugin: (md: StubMd) => void) {
                plugin(md);
                return md;
            },
        };
        governedWikilinkPreviewPlugin(
            md as unknown as Parameters<typeof governedWikilinkPreviewPlugin>[0],
        );
        const firstToken = state.tokens[0];
        return firstToken?.children ?? [];
    }

    test("governedWikilinkPreviewPlugin transforms a governed [[stem]] into an html_inline anchor", () => {
        const children = applyPreviewPlugin("See [[rfc-00015-my-doc]] here.");
        const anchor = children.find((t) => t.type === "html_inline");
        assert.ok(anchor, "must produce an html_inline token");
        assert.ok(
            anchor.content.includes('class="vector-wikilink"'),
            "anchor must carry vector-wikilink class",
        );
        assert.ok(
            anchor.content.includes('data-wikilink="rfc-00015-my-doc"'),
            "anchor must carry data-wikilink attribute",
        );
        assert.ok(anchor.content.includes('href="#"'), "anchor href must be inert");
    });

    test("governedWikilinkPreviewPlugin uses pipe label as anchor text", () => {
        const children = applyPreviewPlugin("[[rfc-00015-my-doc|RFC 15]]");
        const anchor = children.find((t) => t.type === "html_inline");
        assert.ok(anchor, "must produce anchor");
        assert.ok(anchor.content.includes("RFC 15"), "anchor text must be the pipe label");
        assert.ok(
            !anchor.content.includes("rfc-00015-my-doc>"),
            "stem must not appear as text when label provided",
        );
    });

    test("governedWikilinkPreviewPlugin leaves non-governed targets as plain text", () => {
        const children = applyPreviewPlugin("See [[NotGoverned]] here.");
        const hasAnchor = children.some((t) => t.type === "html_inline");
        assert.ok(!hasAnchor, "non-governed target must not produce an anchor");
        const allText = children.map((t) => t.content).join("");
        assert.ok(allText.includes("[[NotGoverned]]"), "raw text must be preserved");
    });

    test("governedWikilinkPreviewPlugin preserves text before and after the wikilink", () => {
        const children = applyPreviewPlugin("Before [[rfc-00001-x]] after.");
        const texts = children.filter((t) => t.type === "text").map((t) => t.content);
        assert.ok(
            texts.some((t) => t.includes("Before")),
            "text before wikilink must be preserved",
        );
        assert.ok(
            texts.some((t) => t.includes("after.")),
            "text after wikilink must be preserved",
        );
    });

    test("governedWikilinkPreviewPlugin handles multiple wikilinks in the same token", () => {
        const children = applyPreviewPlugin("[[rfc-00001-a]] and [[task-00002-b]].");
        const anchors = children.filter((t) => t.type === "html_inline");
        assert.strictEqual(anchors.length, 2, "must produce two anchor tokens");
    });

    test("governedWikilinkPreviewPlugin escapes HTML in anchor text and data-wikilink", () => {
        const children = applyPreviewPlugin("[[rfc-00001-x|A & B <test>]]");
        const anchor = children.find((t) => t.type === "html_inline");
        assert.ok(anchor, "must produce anchor");
        assert.ok(anchor.content.includes("&amp;"), "& must be escaped in label");
        assert.ok(anchor.content.includes("&lt;"), "< must be escaped in label");
    });

    test("governedWikilinkPreviewPlugin does not alter non-text tokens", () => {
        const state = makeState("");
        const nonText = { type: "code_inline", content: "[[rfc-00001-x]]" };
        const firstToken = state.tokens[0];
        assert.ok(firstToken, "first token must exist");
        firstToken.children = [nonText];
        const md: StubMd = {
            core: {
                ruler: {
                    push(_name: string, fn: (s: ReturnType<typeof makeState>) => void) {
                        fn(state);
                    },
                },
            },
            use(plugin: (md: StubMd) => void) {
                plugin(md);
                return md;
            },
        };
        governedWikilinkPreviewPlugin(
            md as unknown as Parameters<typeof governedWikilinkPreviewPlugin>[0],
        );
        const token = state.tokens[0];
        assert.ok(token, "token must exist");
        const children = token.children ?? [];
        assert.strictEqual(children.length, 1, "non-text token must pass through unchanged");
        const firstChild = children[0];
        assert.ok(firstChild, "first child must exist");
        assert.strictEqual(firstChild.type, "code_inline");
    });

    // ── renderGovernedMarkdown with wikilinks ────────────────────────────

    test("renderGovernedMarkdown includes vector-wikilink anchor for governed stems", () => {
        const html = renderGovernedMarkdown("See [[rfc-00015-preview]] for details.");
        assert.ok(
            html.includes('class="vector-wikilink"'),
            "rendered HTML must include wikilink anchor",
        );
        assert.ok(
            html.includes('data-wikilink="rfc-00015-preview"'),
            "anchor must carry governed stem",
        );
    });

    test("renderGovernedMarkdown leaves non-governed wikilinks as plain text", () => {
        const html = renderGovernedMarkdown("See [[SomeThing]] here.");
        assert.ok(
            !html.includes('class="vector-wikilink"'),
            "non-governed target must not become an anchor",
        );
        assert.ok(html.includes("[[SomeThing]]"), "raw wikilink text must appear in output");
    });

    // ── isWikilinkMessage ────────────────────────────────────────────────

    test("isWikilinkMessage returns true for a valid wikilink message", () => {
        const msg = { type: WIKILINK_MESSAGE_TYPE, stem: "rfc-00015-preview" };
        assert.strictEqual(isWikilinkMessage(msg), true);
    });

    test("isWikilinkMessage returns false for wrong type field", () => {
        assert.strictEqual(isWikilinkMessage({ type: "other", stem: "rfc-00001-x" }), false);
    });

    test("isWikilinkMessage returns false when stem is missing", () => {
        assert.strictEqual(isWikilinkMessage({ type: WIKILINK_MESSAGE_TYPE }), false);
    });

    test("isWikilinkMessage returns false for null and non-objects", () => {
        assert.strictEqual(isWikilinkMessage(null), false);
        assert.strictEqual(isWikilinkMessage("string"), false);
        assert.strictEqual(isWikilinkMessage(42), false);
    });

    // ── WIKILINK_CLICK_SCRIPT ─────────────────────────────────────────────

    test("WIKILINK_CLICK_SCRIPT is a non-empty string", () => {
        assert.ok(typeof WIKILINK_CLICK_SCRIPT === "string" && WIKILINK_CLICK_SCRIPT.length > 0);
    });

    test("WIKILINK_CLICK_SCRIPT references the correct message type", () => {
        assert.ok(
            WIKILINK_CLICK_SCRIPT.includes(WIKILINK_MESSAGE_TYPE),
            "click script must reference the correct message type constant",
        );
    });

    test("WIKILINK_CLICK_SCRIPT references data-wikilink and postMessage", () => {
        assert.ok(
            WIKILINK_CLICK_SCRIPT.includes("data-wikilink"),
            "script must read data-wikilink",
        );
        assert.ok(WIKILINK_CLICK_SCRIPT.includes("postMessage"), "script must call postMessage");
    });

    // ── buildPreviewHtml with inline script ───────────────────────────────

    test("buildPreviewHtml injects stylesheet and script asset tags", () => {
        const html = buildPreviewHtml(TEST_WEBVIEW, "Test", "<p>body</p>", TEST_PREVIEW_ASSETS);
        assert.ok(
            html.includes(`href="${TEST_PREVIEW_ASSETS.styleUri}"`),
            "stylesheet URI must appear in output",
        );
        assert.ok(
            html.includes(`src="${TEST_PREVIEW_ASSETS.scriptUri}"`),
            "script URI must appear in output",
        );
    });

    test("buildPreviewHtml CSP keeps inline styles disabled by default", () => {
        const html = buildPreviewHtml(TEST_WEBVIEW, "Test", "<p>body</p>", TEST_PREVIEW_ASSETS);
        assert.ok(
            html.includes("style-src vscode-webview-resource:"),
            "CSP must allow webview-hosted styles",
        );
        assert.ok(
            html.includes("script-src vscode-webview-resource:"),
            "CSP must allow only webview-hosted scripts",
        );
        assert.ok(
            !html.includes("script-src vscode-webview-resource: 'unsafe-inline'"),
            "CSP must keep inline scripts disabled",
        );
    });

    test("buildPreviewHtml CSP always allows inline styles for CodeMirror overlay support", () => {
        const html = buildPreviewHtml(TEST_WEBVIEW, "Test", "<p>body</p>", TEST_PREVIEW_ASSETS);
        assert.ok(
            html.includes("style-src vscode-webview-resource: 'unsafe-inline'"),
            "CSP must always allow inline styles required by CodeMirror",
        );
        assert.ok(
            !html.includes("script-src vscode-webview-resource: 'unsafe-inline'"),
            "CSP must keep inline scripts disabled",
        );
    });

    test("buildPreviewHtml references the shared preview stylesheet", () => {
        const html = buildPreviewHtml(TEST_WEBVIEW, "Test", "", TEST_PREVIEW_ASSETS);
        assert.ok(html.includes("preview.css"), "shell must reference the preview stylesheet");
    });
});

suite("Phase C.5 (RFC 00015) — Frontmatter Properties Panel", () => {
    // ── splitFrontmatter — extraction ────────────────────────────────────

    test("splitFrontmatter returns empty fields and full content when no frontmatter present", () => {
        const { fields, body } = splitFrontmatter("# Hello\n\nParagraph.");
        assert.deepStrictEqual(fields, {});
        assert.strictEqual(body, "# Hello\n\nParagraph.");
    });

    test("splitFrontmatter extracts scalar fields from a well-formed frontmatter block", () => {
        const content = "---\ntitle: My RFC\nstatus: draft\n---\n\n# Body\n";
        const { fields, body } = splitFrontmatter(content);
        assert.strictEqual(fields["title"], "My RFC");
        assert.strictEqual(fields["status"], "draft");
        assert.ok(body.includes("# Body"), "body must contain the markdown content");
        assert.ok(!body.includes("---"), "body must not contain the frontmatter fences");
    });

    test("splitFrontmatter parses quoted string values", () => {
        const content = '---\ntitle: "Quoted Title"\n---\n';
        const { fields } = splitFrontmatter(content);
        assert.strictEqual(fields["title"], "Quoted Title");
    });

    test("splitFrontmatter parses block sequence into an array", () => {
        const content = "---\ntags:\n  - vscode\n  - preview\n---\n";
        const { fields } = splitFrontmatter(content);
        assert.deepStrictEqual(fields["tags"], ["vscode", "preview"]);
    });

    test("splitFrontmatter parses boolean values", () => {
        const content = "---\nenabled: true\ndisabled: false\n---\n";
        const { fields } = splitFrontmatter(content);
        assert.strictEqual(fields["enabled"], true);
        assert.strictEqual(fields["disabled"], false);
    });

    test("splitFrontmatter parses null values", () => {
        const content = "---\nsuperseded_by: null\n---\n";
        const { fields } = splitFrontmatter(content);
        assert.strictEqual(fields["superseded_by"], null);
    });

    test("splitFrontmatter returns empty fields and original content for malformed YAML fences", () => {
        const content = "Not starting with ---\n# Body";
        const { fields, body } = splitFrontmatter(content);
        assert.deepStrictEqual(fields, {});
        assert.strictEqual(body, content);
    });

    test("splitFrontmatter handles CRLF line endings in frontmatter", () => {
        const content = "---\r\ntitle: CRLF Doc\r\n---\r\n\r\n# Body\r\n";
        const { fields, body } = splitFrontmatter(content);
        assert.strictEqual(fields["title"], "CRLF Doc");
        assert.ok(body.includes("# Body"));
    });

    test("splitFrontmatter body does not include a leading blank line from the fence", () => {
        const content = "---\ntitle: T\n---\n# Heading\n";
        const { body } = splitFrontmatter(content);
        assert.ok(!body.startsWith("\n"), "body must not begin with a spurious blank line");
    });

    // ── renderFrontmatterPanel — HTML output ─────────────────────────────

    test("renderFrontmatterPanel returns empty string when fields is empty", () => {
        assert.strictEqual(renderFrontmatterPanel({}), "");
    });

    test("renderFrontmatterPanel wraps output in vector-frontmatter div", () => {
        const html = renderFrontmatterPanel({ title: "My RFC" });
        assert.ok(html.includes('class="vector-frontmatter"'), "must wrap in vector-frontmatter");
    });

    test("renderFrontmatterPanel renders each key as a row", () => {
        const html = renderFrontmatterPanel({ title: "T", status: "draft" });
        assert.ok(html.includes("vector-fm-row"), "must contain row elements");
        assert.ok(html.includes("title"), "must render title key");
        assert.ok(html.includes("status"), "must render status key");
    });

    test("renderFrontmatterPanel renders scalar strings with vector-fm-scalar class", () => {
        const html = renderFrontmatterPanel({ title: "My RFC" });
        assert.ok(html.includes('class="vector-fm-scalar"'), "scalar must use vector-fm-scalar");
        assert.ok(html.includes("My RFC"), "scalar value must appear in output");
    });

    test("renderFrontmatterPanel renders array values as chips", () => {
        const html = renderFrontmatterPanel({ tags: ["vscode", "preview"] });
        assert.ok(html.includes('class="vector-fm-chip"'), "array items must render as chips");
        assert.ok(html.includes("vscode"), "first tag must appear");
        assert.ok(html.includes("preview"), "second tag must appear");
    });

    test("renderFrontmatterPanel renders boolean values as chips", () => {
        const html = renderFrontmatterPanel({ enabled: true });
        assert.ok(html.includes('class="vector-fm-chip"'), "boolean must render as chip");
        assert.ok(html.includes("true"), "boolean value must appear");
    });

    test("renderFrontmatterPanel renders null values as Empty placeholder", () => {
        const html = renderFrontmatterPanel({ superseded_by: null });
        assert.ok(
            html.includes('class="vector-fm-empty"'),
            "null must render as empty placeholder",
        );
        assert.ok(html.includes("Empty"), "empty placeholder text must appear");
    });

    test("renderFrontmatterPanel renders empty array as Empty placeholder", () => {
        const html = renderFrontmatterPanel({ supersedes: [] });
        assert.ok(
            html.includes('class="vector-fm-empty"'),
            "empty array must render as empty placeholder",
        );
    });

    test("renderFrontmatterPanel renders ISO date strings with vector-fm-date class", () => {
        const html = renderFrontmatterPanel({ created: "2026-05-08" });
        assert.ok(
            html.includes('class="vector-fm-date"'),
            "ISO date must use vector-fm-date class",
        );
        assert.ok(html.includes("2026-05-08"), "date value must appear in output");
    });

    test("renderFrontmatterPanel escapes HTML special characters in keys and values", () => {
        const html = renderFrontmatterPanel({ "<key>": '<value & "quoted">' });
        assert.ok(!html.includes("<key>"), "key < must be escaped");
        assert.ok(!html.includes("<value"), "value < must be escaped");
        assert.ok(html.includes("&lt;key&gt;"), "key must be HTML-encoded");
        assert.ok(html.includes("&amp;"), "& must be HTML-encoded");
    });

    // ── buildPreviewHtml integration ─────────────────────────────────────

    test("buildPreviewHtml injects frontmatterHtml before the body when provided", () => {
        const fm = '<div class="vector-frontmatter">props</div>';
        const html = buildPreviewHtml(TEST_WEBVIEW, "Test", "<p>body</p>", TEST_PREVIEW_ASSETS, fm);
        const fmPos = html.indexOf("vector-frontmatter");
        const bodyPos = html.indexOf("<p>body</p>");
        assert.ok(fmPos !== -1, "frontmatter section must appear in output");
        assert.ok(bodyPos !== -1, "body must appear in output");
        assert.ok(fmPos < bodyPos, "frontmatter must precede the body");
    });

    test("buildPreviewHtml omits frontmatter section when frontmatterHtml is not provided", () => {
        const html = buildPreviewHtml(TEST_WEBVIEW, "Test", "<p>body</p>", TEST_PREVIEW_ASSETS);
        assert.ok(
            !html.includes('<div class="vector-frontmatter"'),
            "no frontmatter div must appear when omitted",
        );
    });

    test("buildPreviewHtml links the shared preview stylesheet for frontmatter rendering", () => {
        const html = buildPreviewHtml(TEST_WEBVIEW, "Test", "", TEST_PREVIEW_ASSETS);
        assert.ok(html.includes("preview.css"), "shell must link the shared preview stylesheet");
    });

    // ── end-to-end: splitFrontmatter → renderFrontmatterPanel ────────────

    test("full round-trip renders a task frontmatter block as a panel with all field types", () => {
        const content = [
            "---",
            "id: task-00022-sample",
            'title: "Sample Task"',
            "status: todo",
            "created: 2026-05-08",
            "updated: 2026-05-09",
            "tags:",
            "  - vscode",
            "  - preview",
            "superseded_by: null",
            "---",
            "",
            "# Body",
        ].join("\n");

        const { fields, body } = splitFrontmatter(content);
        const panel = renderFrontmatterPanel(fields);

        assert.ok(panel.includes("vector-frontmatter"), "panel must be wrapped");
        assert.ok(panel.includes("task-00022-sample"), "id must appear");
        assert.ok(panel.includes("Sample Task"), "title must appear");
        assert.ok(panel.includes("vector-fm-date"), "date fields must use date class");
        assert.ok(panel.includes("vector-fm-chip"), "tags must render as chips");
        assert.ok(panel.includes("vector-fm-empty"), "null field must render as empty");
        assert.ok(body.trim().startsWith("# Body"), "body must begin with the heading");
    });
});

suite("Phase C.6 (RFC 00015) — Frontmatter Document Links", () => {
    // ── buildFmLinkAnchor ────────────────────────────────────────────────

    test("buildFmLinkAnchor produces an anchor with data-fmlink and vector-fm-link class", () => {
        const html = buildFmLinkAnchor("rfc-00015-preview");
        assert.ok(html.includes('class="vector-fm-link"'), "must carry vector-fm-link class");
        assert.ok(
            html.includes('data-fmlink="rfc-00015-preview"'),
            "must carry data-fmlink attribute",
        );
        assert.ok(html.includes('href="#"'), "href must be inert");
        assert.ok(html.includes("rfc-00015-preview"), "stem must appear as anchor text");
    });

    test("buildFmLinkAnchor escapes HTML special characters in the stem", () => {
        const html = buildFmLinkAnchor("rfc-00001-a&b");
        assert.ok(html.includes("&amp;"), "& must be HTML-escaped");
        assert.ok(!html.includes("a&b"), "unescaped & must not appear");
    });

    // ── renderFrontmatterPanel — linkification ───────────────────────────

    test("renderFrontmatterPanel linkifies a scalar value that matches the governed stem pattern", () => {
        const html = renderFrontmatterPanel({ related: "rfc-00014-sidebar" });
        assert.ok(html.includes('class="vector-fm-link"'), "governed scalar must become a fm-link");
        assert.ok(html.includes('data-fmlink="rfc-00014-sidebar"'), "fm-link must carry the stem");
    });

    test("renderFrontmatterPanel linkifies governed stems inside array values", () => {
        const html = renderFrontmatterPanel({ related: ["rfc-00014-sidebar", "task-00020-impl"] });
        const linkCount = (html.match(/class="vector-fm-link"/g) ?? []).length;
        assert.strictEqual(linkCount, 2, "both governed array items must become fm-links");
    });

    test("renderFrontmatterPanel does NOT linkify the id field even when it matches a governed stem", () => {
        const html = renderFrontmatterPanel({ id: "rfc-00015-preview" });
        assert.ok(!html.includes("data-fmlink"), "id field must never produce a fm-link");
        assert.ok(html.includes("rfc-00015-preview"), "id value must still appear as text");
    });

    test("renderFrontmatterPanel does NOT linkify the slug field", () => {
        const html = renderFrontmatterPanel({ slug: "task-00022-implement" });
        assert.ok(!html.includes("data-fmlink"), "slug field must never produce a fm-link");
    });

    test("renderFrontmatterPanel does not linkify plain scalar values that are not governed stems", () => {
        const html = renderFrontmatterPanel({ status: "draft", title: "My RFC" });
        assert.ok(
            !html.includes('class="vector-fm-link"'),
            "non-stem values must not become fm-links",
        );
    });

    test("renderFrontmatterPanel does not linkify date-like values", () => {
        const html = renderFrontmatterPanel({ created: "2026-05-08" });
        assert.ok(!html.includes('class="vector-fm-link"'), "date values must not become fm-links");
        assert.ok(html.includes('class="vector-fm-date"'), "date must still use date class");
    });

    test("renderFrontmatterPanel does not linkify non-governed array items", () => {
        const html = renderFrontmatterPanel({ tags: ["vscode", "preview"] });
        assert.ok(
            !html.includes('class="vector-fm-link"'),
            "non-stem array items must not become fm-links",
        );
        assert.ok(
            html.includes('class="vector-fm-chip"'),
            "non-stem items must still render as chips",
        );
    });

    test("renderFrontmatterPanel mixed array: linkifies governed items and leaves others as chips", () => {
        const html = renderFrontmatterPanel({ related: ["rfc-00001-spec", "plain-tag"] });
        assert.ok(html.includes('class="vector-fm-link"'), "governed item must be a fm-link");
        assert.ok(html.includes('class="vector-fm-chip"'), "plain item must remain a chip");
    });

    // ── isFmLinkMessage ──────────────────────────────────────────────────

    test("isFmLinkMessage returns true for a valid fm-link message", () => {
        const msg = { type: FM_LINK_MESSAGE_TYPE, stem: "rfc-00015-preview" };
        assert.strictEqual(isFmLinkMessage(msg), true);
    });

    test("isFmLinkMessage returns false for a wikilink message type", () => {
        assert.strictEqual(
            isFmLinkMessage({ type: "vector.navigateWikilink", stem: "rfc-00001-x" }),
            false,
        );
    });

    test("isFmLinkMessage returns false when stem is missing", () => {
        assert.strictEqual(isFmLinkMessage({ type: FM_LINK_MESSAGE_TYPE }), false);
    });

    test("isFmLinkMessage returns false for null and non-objects", () => {
        assert.strictEqual(isFmLinkMessage(null), false);
        assert.strictEqual(isFmLinkMessage("string"), false);
        assert.strictEqual(isFmLinkMessage(42), false);
    });

    // ── WIKILINK_CLICK_SCRIPT covers fm-link dispatch ────────────────────

    test("WIKILINK_CLICK_SCRIPT references FM_LINK_MESSAGE_TYPE", () => {
        assert.ok(
            WIKILINK_CLICK_SCRIPT.includes(FM_LINK_MESSAGE_TYPE),
            "click script must reference the FM link message type",
        );
    });

    test("WIKILINK_CLICK_SCRIPT references data-fmlink attribute", () => {
        assert.ok(
            WIKILINK_CLICK_SCRIPT.includes("data-fmlink"),
            "click script must read data-fmlink for fm-link dispatch",
        );
    });

    // ── previewHtml CSS ───────────────────────────────────────────────────

    test("buildPreviewHtml links the shared preview stylesheet for frontmatter links", () => {
        const html = buildPreviewHtml(TEST_WEBVIEW, "Test", "", TEST_PREVIEW_ASSETS);
        assert.ok(html.includes("preview.css"), "shell must link the shared preview stylesheet");
    });
});

suite("Phase D (RFC 00015) — Callouts, Code Presentation, and Tables", () => {
    test("governedCalloutPlugin transforms a governed callout blockquote into a dedicated container", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("> [!NOTE] Read Me\n> Body text.");

        assert.ok(
            html.includes('class="vector-callout vector-callout--note"'),
            "callout container class must be present",
        );
        assert.ok(
            html.includes('data-callout-type="note"'),
            "callout type attribute must be present",
        );
        assert.ok(
            html.includes('class="vector-callout-title"'),
            "callout title row must be rendered",
        );
        assert.ok(
            html.includes('class="vector-callout-label">NOTE<'),
            "callout label must be rendered",
        );
        assert.ok(
            html.includes('class="vector-callout-heading">Read Me<'),
            "callout title must be rendered",
        );
        assert.ok(
            html.includes("<p>Body text.</p>"),
            "callout body must remain rendered as markdown",
        );
    });

    test("governedCalloutPlugin preserves same-paragraph body text after the title line", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("> [!TIP] Fast path\n> Keep going on the next line.");

        assert.ok(html.includes("vector-callout--tip"), "tip callout class must be rendered");
        assert.ok(
            html.includes("<p>Keep going on the next line.</p>"),
            "body text after the title line must be preserved",
        );
        assert.ok(!html.includes("[!TIP]"), "callout marker must not leak into rendered HTML");
    });

    test("governedCalloutPlugin leaves a plain blockquote untouched", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("> Plain quote");

        assert.ok(
            html.includes("<blockquote>"),
            "plain blockquote must still render as a blockquote",
        );
        assert.ok(!html.includes("vector-callout"), "plain blockquote must not become a callout");
    });

    test("renderGovernedMarkdown escapes HTML in callout titles", () => {
        const html = renderGovernedMarkdown("> [!WARNING] <script>alert(1)</script>\n> Safe body");

        assert.ok(
            html.includes("&lt;script&gt;alert(1)&lt;/script&gt;"),
            "title HTML must be escaped",
        );
        assert.ok(!html.includes("<script>"), "raw script tag must not appear in output");
    });

    test("buildPreviewHtml links the shared preview stylesheet for callouts", () => {
        const html = buildPreviewHtml(TEST_WEBVIEW, "Test", "", TEST_PREVIEW_ASSETS);
        assert.ok(html.includes("preview.css"), "shell must link the shared preview stylesheet");
    });
});

suite("Task 00028 Phase B — form-editor Module", () => {
    // ── parseFormBlock ────────────────────────────────────────────────────

    test("parseFormBlock parses a single input field", () => {
        const fields = parseFormBlock('name = input("Your Name")');
        assert.strictEqual(fields.length, 1);
        const f = fields[0];
        assert.ok(f !== undefined, "first field must exist");
        assert.strictEqual(f.key, "name");
        assert.strictEqual(f.type, "input");
        assert.strictEqual(f.label, "Your Name");
        assert.strictEqual(f.value, null);
        assert.strictEqual(f.readOnly, false);
    });

    test("parseFormBlock parses a chat-input field", () => {
        const fields = parseFormBlock('bio = chat-input("Tell us about yourself")');
        assert.strictEqual(fields.length, 1);
        const f = fields[0];
        assert.ok(f !== undefined, "first field must exist");
        assert.strictEqual(f.key, "bio");
        assert.strictEqual(f.type, "chat-input");
        assert.strictEqual(f.label, "Tell us about yourself");
        assert.strictEqual(f.readOnly, false);
    });

    test("parseFormBlock treats unquoted value as read-only pre-filled field", () => {
        const fields = parseFormBlock("doc-type = input(rfc)");
        assert.strictEqual(fields.length, 1);
        const f = fields[0];
        assert.ok(f !== undefined, "first field must exist");
        assert.strictEqual(f.key, "doc-type");
        assert.strictEqual(f.readOnly, true);
        assert.strictEqual(f.value, "rfc");
        assert.strictEqual(f.label, null);
    });

    test("parseFormBlock parses multiple fields in document order", () => {
        const src = [
            'title = input("Document Title")',
            'description = chat-input("Description")',
            "phase = input(Phase A)",
        ].join("\n");
        const fields = parseFormBlock(src);
        assert.strictEqual(fields.length, 3);
        const [f0, f1, f2] = fields;
        assert.ok(f0 !== undefined && f1 !== undefined && f2 !== undefined);
        assert.strictEqual(f0.key, "title");
        assert.strictEqual(f1.key, "description");
        assert.strictEqual(f2.key, "phase");
        assert.strictEqual(f2.readOnly, true);
    });

    test("parseFormBlock silently ignores blank lines", () => {
        const src = '\ntitle = input("Title")\n\nbio = chat-input("Bio")\n';
        const fields = parseFormBlock(src);
        assert.strictEqual(fields.length, 2);
    });

    test("parseFormBlock silently ignores lines that do not match the grammar", () => {
        const src = ["# comment", "invalid line", 'name = input("Name")'].join("\n");
        const fields = parseFormBlock(src);
        assert.strictEqual(fields.length, 1);
        const f = fields[0];
        assert.ok(f !== undefined, "first field must exist");
        assert.strictEqual(f.key, "name");
    });

    test("parseFormBlock returns empty array for empty content", () => {
        assert.deepStrictEqual(parseFormBlock(""), []);
        assert.deepStrictEqual(parseFormBlock("   \n   "), []);
    });

    test("parseFormBlock unescapes backslash-escaped quotes in labels", () => {
        const fields = parseFormBlock('note = input("Say \\"hello\\"")');
        const f = fields[0];
        assert.ok(f !== undefined, "first field must exist");
        assert.strictEqual(f.label, 'Say "hello"');
    });

    // ── renderFormBlock ───────────────────────────────────────────────────

    test("renderFormBlock produces a vector-form wrapper div", () => {
        const html = renderFormBlock('name = input("Your Name")');
        assert.ok(html.includes('class="vector-form"'), "must render a vector-form wrapper");
    });

    test("renderFormBlock renders an editable input field", () => {
        const html = renderFormBlock('name = input("Your Name")');
        assert.ok(html.includes("<input"), "must render an input element");
        assert.ok(html.includes('type="text"'), "input must be type text");
        assert.ok(html.includes('name="name"'), "input must carry the field key as name");
        assert.ok(!html.includes("placeholder="), "editable input must not render a placeholder");
        assert.ok(
            !html.includes("vector-form-readonly-value"),
            "editable field must not be readonly",
        );
    });

    test("renderFormBlock renders an editable chat-input as editor host", () => {
        const html = renderFormBlock('bio = chat-input("Biography")');
        assert.ok(
            html.includes('class="vector-chat-input-host"'),
            "chat-input must render the editor host container",
        );
        assert.ok(
            html.includes('data-chat-input-name="bio"'),
            "host must carry the field key as data-chat-input-name",
        );
        assert.ok(
            !html.includes("data-placeholder="),
            "chat-input host must not render a placeholder marker",
        );
        assert.ok(
            !html.includes('class="vector-form-input vector-form-textarea"'),
            "chat-input must not use the old textarea classes",
        );
    });

    test("renderFormBlock renders a read-only pre-filled field", () => {
        const html = renderFormBlock("phase = input(Phase A)");
        assert.ok(
            html.includes('class="vector-form-readonly-value"'),
            "read-only field must use readonly-value span",
        );
        assert.ok(html.includes("Phase A"), "pre-filled value must appear in output");
        assert.ok(!html.includes("<input"), "read-only field must not render an input element");
    });

    test("renderFormBlock returns empty string when no valid fields are parsed", () => {
        const html = renderFormBlock("# not a valid field\n   ");
        assert.strictEqual(html, "", "must return empty string for unrecognised content");
    });

    test("renderFormBlock escapes HTML in field keys and values", () => {
        const html = renderFormBlock('<key> = input("<script>")');
        assert.ok(!html.includes("<key>"), "field key must be escaped in output");
        assert.ok(!html.includes("<script>"), "label must be escaped in output");
    });

    test("renderFormBlock sets data-form-key and data-form-type attributes", () => {
        const html = renderFormBlock('title = chat-input("Title")');
        assert.ok(html.includes('data-form-key="title"'), "data-form-key must be set");
        assert.ok(html.includes('data-form-type="chat-input"'), "data-form-type must be set");
    });

    // ── integration with markdownRenderer ────────────────────────────────

    test("createGovernedMarkdownIt renders vector-form block as a form, not a code block", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render('```vector-form\nname = input("Name")\n```');
        assert.ok(html.includes('class="vector-form"'), "vector-form must render as a form");
        assert.ok(
            !html.includes("vector-code-block"),
            "vector-form must not render as a code block",
        );
    });

    test("createGovernedMarkdownIt does not syntax-highlight vector-form blocks", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render('```vector-form\nname = input("Name")\n```');
        assert.ok(
            !html.includes('class="language-vector-form"'),
            "vector-form must not get a language class",
        );
    });

    test("renderGovernedMarkdown renders multiple vector-form blocks in a document", () => {
        const src = [
            "```vector-form",
            'name = input("Name")',
            "```",
            "",
            "```vector-form",
            'bio = chat-input("Bio")',
            "```",
        ].join("\n");
        const html = renderGovernedMarkdown(src);
        const formCount = (html.match(/class="vector-form"/g) ?? []).length;
        assert.strictEqual(formCount, 2, "both vector-form blocks must be rendered");
    });
});

suite("Task 00028 Phase C — document-actions: vector-open-doc", () => {
    // ── substituteVariables ───────────────────────────────────────────────

    test("substituteVariables replaces a single placeholder", () => {
        assert.strictEqual(
            substituteVariables("Hello #{name}!", { name: "World" }),
            "Hello World!",
        );
    });

    test("substituteVariables replaces multiple placeholders in one pass", () => {
        const result = substituteVariables("#{a} and #{b}", { a: "foo", b: "bar" });
        assert.strictEqual(result, "foo and bar");
    });

    test("substituteVariables replaces the same placeholder appearing multiple times", () => {
        const result = substituteVariables("#{x} #{x} #{x}", { x: "y" });
        assert.strictEqual(result, "y y y");
    });

    test("substituteVariables leaves unknown placeholders unchanged", () => {
        const result = substituteVariables("#{known} #{unknown}", { known: "ok" });
        assert.strictEqual(result, "ok #{unknown}");
    });

    test("substituteVariables returns the input unchanged when variables map is empty", () => {
        const src = "no placeholders here";
        assert.strictEqual(substituteVariables(src, {}), src);
    });

    test("substituteVariables replaces kebab-case names and leaves underscore placeholders unresolved", () => {
        const result = substituteVariables("#{doc-type} #{doc_type}", {
            "doc-type": "rfc",
            doc_type: "task",
        });
        assert.strictEqual(result, "rfc #{doc_type}");
    });

    test("substituteVariables does not modify text without #{} patterns", () => {
        const src = "plain text with no variables";
        assert.strictEqual(substituteVariables(src, { x: "y" }), src);
    });

    // ── parseOpenDocBlock ─────────────────────────────────────────────────

    test("parseOpenDocBlock parses a minimal valid block", () => {
        const yaml = "label: Open Form\ndoc: form-00001-create-doc\n";
        const result = parseOpenDocBlock(yaml);
        assert.ok(!isOpenDocParseError(result), "must not return an error");
        assert.strictEqual(result.label, "Open Form");
        assert.strictEqual(result.doc, "form-00001-create-doc");
        assert.deepStrictEqual(result.input, {});
    });

    test("parseOpenDocBlock parses a block with input variables", () => {
        const yaml = "label: Create\ndoc: form-00001\ninput:\n  doc-type: rfc\n  language: rust\n";
        const result = parseOpenDocBlock(yaml);
        assert.ok(!isOpenDocParseError(result));
        assert.deepStrictEqual(result.input, { "doc-type": "rfc", language: "rust" });
    });

    test("parseOpenDocBlock returns error when label is missing", () => {
        const result = parseOpenDocBlock("doc: form-00001\n");
        assert.ok(isOpenDocParseError(result), "must return error for missing label");
        assert.ok(result.error.includes("label"), "error must mention 'label'");
    });

    test("parseOpenDocBlock returns error when doc is missing", () => {
        const result = parseOpenDocBlock("label: Open\n");
        assert.ok(isOpenDocParseError(result), "must return error for missing doc");
        assert.ok(result.error.includes("doc"), "error must mention 'doc'");
    });

    test("parseOpenDocBlock returns error for malformed YAML", () => {
        const result = parseOpenDocBlock("label: [unclosed");
        assert.ok(isOpenDocParseError(result), "must return error for invalid YAML");
    });

    test("parseOpenDocBlock returns error when input is not a mapping", () => {
        const result = parseOpenDocBlock("label: L\ndoc: d\ninput:\n  - item\n");
        assert.ok(isOpenDocParseError(result), "must return error when input is a sequence");
    });

    // ── renderOpenDocBlock ────────────────────────────────────────────────

    test("renderOpenDocBlock renders a link with the configured label", () => {
        const html = renderOpenDocBlock("label: Open Form\ndoc: form-00001\n");
        assert.ok(html.includes("Open Form"), "label must appear in output");
        assert.ok(html.includes("<a "), "must render an anchor element");
        assert.ok(html.includes('class="vector-open-doc"'), "must carry the vector-open-doc class");
    });

    test("renderOpenDocBlock sets data-open-doc attribute to the doc identifier", () => {
        const html = renderOpenDocBlock("label: L\ndoc: form-00001-create\n");
        assert.ok(
            html.includes('data-open-doc="form-00001-create"'),
            "data-open-doc must match doc",
        );
    });

    test("renderOpenDocBlock serialises input as JSON in data-open-doc-input", () => {
        const yaml = "label: L\ndoc: d\ninput:\n  type: rfc\n";
        const html = renderOpenDocBlock(yaml);
        assert.ok(html.includes("data-open-doc-input"), "must include data-open-doc-input");
        assert.ok(html.includes("rfc"), "input value must be present in serialised JSON");
    });

    test("renderOpenDocBlock renders an error span for invalid YAML", () => {
        const html = renderOpenDocBlock("label: [broken");
        assert.ok(
            html.includes('class="vector-open-doc-error"'),
            "must render error class on parse failure",
        );
        assert.ok(!html.includes("<a "), "must not render an anchor on error");
    });

    test("renderOpenDocBlock escapes HTML special characters in label and doc", () => {
        const html = renderOpenDocBlock('label: "<click>"\ndoc: x\n');
        assert.ok(!html.includes("<click>"), "HTML in label must be escaped");
    });

    // ── integration with markdownRenderer ────────────────────────────────

    test("createGovernedMarkdownIt renders vector-open-doc block as a link, not a code block", () => {
        const md = createGovernedMarkdownIt();
        const src = "```vector-open-doc\nlabel: Open\ndoc: form-00001\n```";
        const html = md.render(src);
        assert.ok(
            html.includes('class="vector-open-doc"'),
            "must render as a vector-open-doc link",
        );
        assert.ok(!html.includes("vector-code-block"), "must not render as a code block");
    });

    test("renderGovernedMarkdown substitutes #{} placeholders applied from caller context", () => {
        const body = "Hello #{name}!";
        const substituted = substituteVariables(body, { name: "World" });
        const html = renderGovernedMarkdown(substituted);
        assert.ok(
            html.includes("Hello World!"),
            "substituted content must appear in rendered HTML",
        );
        assert.ok(!html.includes("#{name}"), "placeholder must not survive into rendered HTML");
    });
});

suite("Task 00028 Phase D — document-actions: Agent Triggers", () => {
    // ── findUnresolvedVariables ───────────────────────────────────────────

    test("findUnresolvedVariables returns empty array when all placeholders are resolved", () => {
        const result = findUnresolvedVariables("#{a} #{b}", { a: "1", b: "2" });
        assert.deepStrictEqual(result, []);
    });

    test("findUnresolvedVariables returns unique unresolved keys", () => {
        const result = findUnresolvedVariables("#{a} #{b} #{a}", { a: "1" });
        assert.deepStrictEqual(result, ["b"]);
    });

    test("findUnresolvedVariables returns all keys when variables map is empty", () => {
        const result = findUnresolvedVariables("#{x} #{y}", {});
        assert.deepStrictEqual(result, ["x", "y"]);
    });

    test("findUnresolvedVariables rejects underscore placeholders from the unresolved set", () => {
        const result = findUnresolvedVariables("#{doc_type} #{doc-type}", {});
        assert.deepStrictEqual(result, ["doc-type"]);
    });

    test("findUnresolvedVariables returns empty array for text without placeholders", () => {
        assert.deepStrictEqual(findUnresolvedVariables("no placeholders", { x: "1" }), []);
    });

    // ── parseAgentBlock ───────────────────────────────────────────────────

    test("parseAgentBlock parses a fully specified block", () => {
        const yaml =
            "label: Execute\nprofile: create-doc\nprompt: prompt-00003-create\ninput:\n  phase: A\n";
        const result = parseAgentBlock(yaml);
        assert.ok(!isAgentBlockParseError(result));
        assert.strictEqual(result.label, "Execute");
        assert.strictEqual(result.profile, "create-doc");
        assert.strictEqual(result.prompt, "prompt-00003-create");
        assert.deepStrictEqual(result.input, { phase: "A" });
    });

    test("parseAgentBlock parses a minimal block (no input)", () => {
        const yaml = "label: Run\nprofile: code\nprompt: prompt-00001\n";
        const result = parseAgentBlock(yaml);
        assert.ok(!isAgentBlockParseError(result));
        assert.deepStrictEqual(result.input, {});
    });

    test("parseAgentBlock returns error when label is missing", () => {
        const result = parseAgentBlock("profile: code\nprompt: p\n");
        assert.ok(isAgentBlockParseError(result));
        assert.ok(result.error.includes("label"));
    });

    test("parseAgentBlock returns error when profile is missing", () => {
        const result = parseAgentBlock("label: L\nprompt: p\n");
        assert.ok(isAgentBlockParseError(result));
        assert.ok(result.error.includes("profile"));
    });

    test("parseAgentBlock returns error when prompt is missing", () => {
        const result = parseAgentBlock("label: L\nprofile: p\n");
        assert.ok(isAgentBlockParseError(result));
        assert.ok(result.error.includes("prompt"));
    });

    test("parseAgentBlock returns error for malformed YAML", () => {
        const result = parseAgentBlock("label: [broken");
        assert.ok(isAgentBlockParseError(result));
    });

    // ── renderAgentBlock ──────────────────────────────────────────────────

    test("renderAgentBlock (button) renders a button with vector-agent-button class", () => {
        const yaml = "label: Execute\nprofile: p\nprompt: q\n";
        const html = renderAgentBlock(yaml, "button");
        assert.ok(html.includes("<button"), "must render a button element");
        assert.ok(
            html.includes('class="vector-agent-button"'),
            "must carry vector-agent-button class",
        );
        assert.ok(html.includes("Execute"), "label must appear in content");
    });

    test("renderAgentBlock (action) renders a button with vector-agent-action class", () => {
        const yaml = "label: Go\nprofile: p\nprompt: q\n";
        const html = renderAgentBlock(yaml, "action");
        assert.ok(
            html.includes('class="vector-agent-action"'),
            "must carry vector-agent-action class",
        );
    });

    test("renderAgentBlock sets data-agent-profile and data-agent-prompt attributes", () => {
        const yaml = "label: L\nprofile: create-doc\nprompt: prompt-00003\n";
        const html = renderAgentBlock(yaml, "button");
        assert.ok(html.includes('data-agent-profile="create-doc"'));
        assert.ok(html.includes('data-agent-prompt="prompt-00003"'));
    });

    test("renderAgentBlock serialises input as JSON in data-agent-input", () => {
        const yaml = "label: L\nprofile: p\nprompt: q\ninput:\n  phase: A\n";
        const html = renderAgentBlock(yaml, "button");
        assert.ok(html.includes("data-agent-input"), "must include data-agent-input");
        assert.ok(html.includes("phase"), "input key must be serialised");
    });

    test("renderAgentBlock renders an error span for invalid YAML", () => {
        const html = renderAgentBlock("label: [broken", "button");
        assert.ok(html.includes('class="vector-agent-error"'));
        assert.ok(!html.includes("<button"), "must not render a button on error");
    });

    test("renderAgentBlock escapes HTML in label", () => {
        const yaml = 'label: "<script>"\nprofile: p\nprompt: q\n';
        const html = renderAgentBlock(yaml, "button");
        assert.ok(!html.includes("<script>"), "HTML in label must be escaped");
    });

    // ── markdownRenderer integration ──────────────────────────────────────

    test("createGovernedMarkdownIt renders vector-agent-button as a button, not a code block", () => {
        const md = createGovernedMarkdownIt();
        const src = "```vector-agent-button\nlabel: Run\nprofile: p\nprompt: q\n```";
        const html = md.render(src);
        assert.ok(html.includes('class="vector-agent-button"'), "must render as agent button");
        assert.ok(!html.includes("vector-code-block"), "must not render as code block");
    });

    test("createGovernedMarkdownIt renders vector-agent-action as a flat action button", () => {
        const md = createGovernedMarkdownIt();
        const src = "```vector-agent-action\nlabel: Go\nprofile: p\nprompt: q\n```";
        const html = md.render(src);
        assert.ok(html.includes('class="vector-agent-action"'), "must render as agent action");
        assert.ok(!html.includes("vector-code-block"), "must not render as code block");
    });

    // ── loadAgentsConfig ──────────────────────────────────────────────────

    function makeTempDir(): string {
        return fs.mkdtempSync(path.join(os.tmpdir(), "vector-agents-test-"));
    }

    const VALID_AGENTS_YAML = [
        "agents:",
        "  claude:",
        "    type: cli",
        '    command: claude "$(cat <file>)"',
        "  codex:",
        "    type: cli",
        '    command: codex "$(cat <file>)"',
        "profiles:",
        "  create-doc: [claude, codex]",
        "  code: [claude]",
    ].join("\n");

    test("loadAgentsConfig returns missing=true when .vector/agents.yaml is absent", () => {
        const dir = makeTempDir();
        try {
            const result = loadAgentsConfig(dir);
            assert.ok(!result.ok && result.missing, "must report missing when file is absent");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("loadAgentsConfig parses a valid agents.yaml", () => {
        const dir = makeTempDir();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(path.join(dir, ".vector", "agents.yaml"), VALID_AGENTS_YAML, "utf-8");
            const result = loadAgentsConfig(dir);
            assert.ok(result.ok, "must succeed for valid YAML");
            assert.ok(result.config.agents.claude, "must parse claude agent");
            assert.strictEqual(result.config.agents.claude.command, 'claude "$(cat <file>)"');
            assert.ok(Array.isArray(result.config.profiles["create-doc"]));
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("loadAgentsConfig returns error for malformed YAML", () => {
        const dir = makeTempDir();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(
                path.join(dir, ".vector", "agents.yaml"),
                "agents: [bad: yaml",
                "utf-8",
            );
            const result = loadAgentsConfig(dir);
            assert.ok(!result.ok && !result.missing, "must report error for bad YAML");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("loadAgentsConfig returns error when an agent command is empty", () => {
        const yaml = [
            "agents:",
            "  claude:",
            '    command: "   "',
            "profiles:",
            "  code: [claude]",
        ].join("\n");
        const dir = makeTempDir();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(path.join(dir, ".vector", "agents.yaml"), yaml, "utf-8");
            const result = loadAgentsConfig(dir);
            assert.ok(!result.ok && !result.missing, "must report invalid command");
            assert.match(result.error, /command must not be empty/);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("loadAgentsConfig rejects snake_case schema fields", () => {
        const yaml = [
            "agents:",
            "  claude:",
            "    prompt_template: prompts-00004-execute-task-phase",
            '    command: claude "$(cat <file>)"',
            "profiles:",
            "  code: [claude]",
        ].join("\n");
        const dir = makeTempDir();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(path.join(dir, ".vector", "agents.yaml"), yaml, "utf-8");
            const result = loadAgentsConfig(dir);
            assert.ok(!result.ok && !result.missing, "must report invalid schema field");
            assert.match(
                result.error,
                /\.vector\/agents\.yaml: invalid YAML field 'prompt_template'/,
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("loadAgentsConfig rejects snake_case top-level schema fields", () => {
        const yaml = [
            "agents:",
            "  claude:",
            '    command: claude "$(cat <file>)"',
            "agent_profiles:",
            "  code: [claude]",
        ].join("\n");
        const dir = makeTempDir();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(path.join(dir, ".vector", "agents.yaml"), yaml, "utf-8");
            const result = loadAgentsConfig(dir);
            assert.ok(!result.ok && !result.missing, "must report invalid top-level field");
            assert.match(
                result.error,
                /\.vector\/agents\.yaml: invalid YAML field 'agent_profiles'/,
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
    test("loadAgentsConfig returns error when an agent command omits <file>", () => {
        const yaml = [
            "agents:",
            "  claude:",
            "    command: claude",
            "profiles:",
            "  code: [claude]",
        ].join("\n");
        const dir = makeTempDir();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(path.join(dir, ".vector", "agents.yaml"), yaml, "utf-8");
            const result = loadAgentsConfig(dir);
            assert.ok(!result.ok && !result.missing, "must report invalid command");
            assert.match(result.error, /must include the <file> placeholder/);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    // ── resolveProfile ────────────────────────────────────────────────────

    test("resolveProfile returns agents in profile order with injected availability", () => {
        const dir = makeTempDir();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(path.join(dir, ".vector", "agents.yaml"), VALID_AGENTS_YAML, "utf-8");
            const load = loadAgentsConfig(dir);
            assert.ok(load.ok);
            const agents = resolveProfile(load.config, "create-doc", () => true);
            assert.strictEqual(agents.length, 2);
            const first = agents[0];
            assert.ok(first !== undefined);
            assert.strictEqual(first.name, "claude");
            assert.strictEqual(first.available, true);
            assert.strictEqual(agents[1]?.name, "codex");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("resolveProfile marks agents unavailable via injected checker", () => {
        const dir = makeTempDir();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(path.join(dir, ".vector", "agents.yaml"), VALID_AGENTS_YAML, "utf-8");
            const load = loadAgentsConfig(dir);
            assert.ok(load.ok);
            const agents = resolveProfile(load.config, "create-doc", () => false);
            assert.ok(
                agents.every((a) => !a.available),
                "all agents must be marked unavailable",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("resolveProfile returns empty array for an unknown profile", () => {
        const dir = makeTempDir();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(path.join(dir, ".vector", "agents.yaml"), VALID_AGENTS_YAML, "utf-8");
            const load = loadAgentsConfig(dir);
            assert.ok(load.ok);
            const agents = resolveProfile(load.config, "nonexistent", () => true);
            assert.deepStrictEqual(agents, []);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("resolveProfile skips agents whose name is not in the agents map", () => {
        const yaml = [
            "agents:",
            "  claude:",
            "    type: cli",
            '    command: claude "$(cat <file>)"',
            "profiles:",
            "  my-profile: [claude, ghost]",
        ].join("\n");
        const dir = makeTempDir();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(path.join(dir, ".vector", "agents.yaml"), yaml, "utf-8");
            const load = loadAgentsConfig(dir);
            assert.ok(load.ok);
            const agents = resolveProfile(load.config, "my-profile", () => true);
            assert.strictEqual(agents.length, 1, "unknown agent 'ghost' must be skipped");
            assert.strictEqual(agents[0]?.name, "claude");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("extractCommandExecutable returns the first token for a templated command", () => {
        assert.strictEqual(extractCommandExecutable('claude "$(cat <file>)"'), "claude");
    });

    test("extractCommandExecutable supports quoted executable paths", () => {
        assert.strictEqual(
            extractCommandExecutable('"C:\\Program Files\\Agent\\agent.exe" --prompt <file>'),
            "C:\\Program Files\\Agent\\agent.exe",
        );
    });

    test("resolveProfile checks availability using the executable, not the full command template", () => {
        const dir = makeTempDir();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(path.join(dir, ".vector", "agents.yaml"), VALID_AGENTS_YAML, "utf-8");
            const load = loadAgentsConfig(dir);
            assert.ok(load.ok);
            const inspected: string[] = [];
            const agents = resolveProfile(load.config, "create-doc", (command) => {
                inspected.push(command);
                return true;
            });
            assert.strictEqual(agents.length, 2);
            assert.deepStrictEqual(inspected, ["claude", "codex"]);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    // ── agent command resolution ─────────────────────────────────────────

    test("quoteShellArgument wraps a path in double quotes", () => {
        assert.strictEqual(
            quoteShellArgument("/tmp/vector prompt.txt"),
            '"/tmp/vector prompt.txt"',
        );
    });

    test("quoteShellArgument escapes shell-sensitive characters inside double quotes", () => {
        const input = 'C:\\temp\\prompt"$`file.txt';
        assert.strictEqual(quoteShellArgument(input), '"C:\\\\temp\\\\prompt\\"\\$\\`file.txt"');
    });

    test("resolveAgentCommand replaces every <file> placeholder with the quoted temp file path", () => {
        const resolved = resolveAgentCommand(
            'claude "$(cat <file>)" && echo <file>',
            "/tmp/vector prompt.txt",
        );
        assert.strictEqual(
            resolved,
            'claude "$(cat "/tmp/vector prompt.txt")" && echo "/tmp/vector prompt.txt"',
        );
    });

    test("resolveAgentCommand throws when the configured command has no <file> placeholder", () => {
        assert.throws(
            () => resolveAgentCommand("claude", "/tmp/vector-prompt.txt"),
            /must include the <file> placeholder/,
        );
    });

    test("spawnAgentTerminal sends the resolved command to the VS Code terminal", () => {
        const tempFilePath = path.join(os.tmpdir(), "vector-agent-terminal-test.txt");
        fs.writeFileSync(tempFilePath, "prompt", "utf-8");

        try {
            vscode.__resetTerminalState();
            const subscriptions: { dispose: () => void }[] = [];

            spawnAgentTerminal(
                'claude "$(cat <file>)"',
                "claude",
                "Execute",
                tempFilePath,
                subscriptions,
            );

            const terminals = vscode.__getCreatedTerminals();
            assert.strictEqual(terminals.length, 1, "must create one terminal");
            const firstTerminal = terminals[0];
            assert.ok(firstTerminal, "terminal record must exist");
            assert.strictEqual(firstTerminal.name, "Vector: claude - Execute");
            assert.deepStrictEqual(firstTerminal.sentText, [
                `claude "$(cat ${quoteShellArgument(tempFilePath)})"`,
            ]);
            assert.deepStrictEqual(
                firstTerminal.showCalls,
                [false],
                "terminal must be shown with preserveFocus=false",
            );
            assert.strictEqual(subscriptions.length, 1, "must register one close subscription");
        } finally {
            fs.rmSync(tempFilePath, { force: true });
            vscode.__resetTerminalState();
        }
    });

    test("spawnAgentTerminal deletes the temp file when the terminal closes", () => {
        const tempFilePath = path.join(os.tmpdir(), "vector-agent-cleanup-test.txt");
        fs.writeFileSync(tempFilePath, "prompt", "utf-8");

        try {
            vscode.__resetTerminalState();
            const subscriptions: { dispose: () => void }[] = [];

            spawnAgentTerminal("claude <file>", "claude", "Execute", tempFilePath, subscriptions);

            const terminal = vscode.__getCreatedTerminals()[0]?.terminal;
            assert.ok(terminal, "terminal must be created");
            assert.ok(fs.existsSync(tempFilePath), "temp file must exist before terminal close");

            vscode.__fireDidCloseTerminal(terminal);

            assert.ok(!fs.existsSync(tempFilePath), "temp file must be deleted on terminal close");
        } finally {
            fs.rmSync(tempFilePath, { force: true });
            vscode.__resetTerminalState();
        }
    });
});

const extensionRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

function createCustomEditorPanel(): {
    panel: import("vscode").WebviewPanel;
    fireMessage(message: unknown): void;
} {
    const stubPanel = vscode.window.createWebviewPanel("vector.documentPreview", "Vector Test");

    return {
        panel: stubPanel as unknown as import("vscode").WebviewPanel,
        fireMessage(message: unknown) {
            vscode.__fireWebviewMessage(stubPanel, message);
        },
    };
}
function openProviderForDoc(
    workspaceRoot: string,
    docPath: string,
): { provider: GovernedDocumentEditorProvider; panel: ReturnType<typeof createCustomEditorPanel> } {
    const provider = new GovernedDocumentEditorProvider(
        workspaceRoot,
        vscode.Uri.file(extensionRoot) as unknown as import("vscode").Uri,
    );
    const panel = createCustomEditorPanel();
    const document = provider.openCustomDocument(
        vscode.Uri.file(docPath) as unknown as import("vscode").Uri,
    );
    provider.resolveCustomEditor(document, panel.panel);
    return { provider, panel };
}

suite("Task 00031 Phase C — Agent Trigger UI Errors", () => {
    function makePreviewWorkspace(): { dir: string; docPath: string } {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-agent-ui-test-"));
        const docPath = path.join(dir, "note.md");
        fs.writeFileSync(docPath, "# Test\n", "utf-8");
        return { dir, docPath };
    }

    test("runAgent shows an error when .vector/agents.yaml is missing", () => {
        const { dir, docPath } = makePreviewWorkspace();
        try {
            vscode.__resetUiState();
            const { panel } = openProviderForDoc(dir, docPath);

            panel.fireMessage({
                type: "vector.runAgent",
                profile: "code",
                prompt: "prompts-00004-execute-task-phase",
                label: "Run",
                staticInput: {},
                formValues: {},
            });

            assert.deepStrictEqual(vscode.__getErrorMessages(), [
                "Vector: .vector/agents.yaml not found — add it to use agent triggers.",
            ]);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
            vscode.__resetUiState();
        }
    });

    test("runAgent shows an error when the requested profile does not exist", () => {
        const { dir, docPath } = makePreviewWorkspace();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(
                path.join(dir, ".vector", "agents.yaml"),
                [
                    "agents:",
                    "  claude:",
                    '    command: claude "$(cat <file>)"',
                    "profiles:",
                    "  other: [claude]",
                ].join("\n"),
                "utf-8",
            );
            vscode.__resetUiState();
            const { panel } = openProviderForDoc(dir, docPath);

            panel.fireMessage({
                type: "vector.runAgent",
                profile: "code",
                prompt: "prompts-00004-execute-task-phase",
                label: "Run",
                staticInput: {},
                formValues: {},
            });

            assert.deepStrictEqual(vscode.__getErrorMessages(), [
                "Vector: profile 'code' not found in .vector/agents.yaml",
            ]);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
            vscode.__resetUiState();
        }
    });

    test("runAgent shows an error when no agents in the profile are installed", () => {
        const { dir, docPath } = makePreviewWorkspace();
        try {
            fs.mkdirSync(path.join(dir, ".vector"));
            fs.writeFileSync(
                path.join(dir, ".vector", "agents.yaml"),
                [
                    "agents:",
                    "  ghost:",
                    "    command: missing-agent --prompt <file>",
                    "profiles:",
                    "  code: [ghost]",
                ].join("\n"),
                "utf-8",
            );
            vscode.__resetUiState();
            const { panel } = openProviderForDoc(dir, docPath);

            panel.fireMessage({
                type: "vector.runAgent",
                profile: "code",
                prompt: "prompts-00004-execute-task-phase",
                label: "Run",
                staticInput: {},
                formValues: {},
            });

            assert.deepStrictEqual(vscode.__getErrorMessages(), [
                "Vector: no agents in profile 'code' are installed (not in PATH: ghost)",
            ]);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
            vscode.__resetUiState();
        }
    });
});

suite("Task 00037 Phase C — File Mention Suggestions", () => {
    function makeMentionWorkspace(): { dir: string; docPath: string } {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-mention-test-"));
        const docPath = path.join(dir, "note.md");
        fs.writeFileSync(docPath, "# Test\n", "utf-8");
        return { dir, docPath };
    }

    test("controller posts FileSuggestionsResult in response to a FileSuggestionsRequest", async () => {
        const { dir, docPath } = makeMentionWorkspace();
        try {
            vscode.__resetUiState();
            vscode.__setFindFilesResults([vscode.Uri.file(path.join(dir, "formRenderer.ts"))]);

            const { panel } = openProviderForDoc(dir, docPath);

            panel.fireMessage({
                type: "vector.chatInput.requestSuggestions",
                requestId: "req-1",
                query: "formRenderer",
            });

            await new Promise<void>((r) => setTimeout(r, 50));

            const posted = vscode.__getPostedMessages(panel.panel);
            const result = posted.find(
                (m): m is { type: string; requestId: string; suggestions: unknown[] } =>
                    typeof m === "object" &&
                    m !== null &&
                    (m as Record<string, unknown>)["type"] === "vector.chatInput.suggestionsResult",
            ) as { type: string; requestId: string; suggestions: unknown[] } | undefined;
            assert.ok(result, "a suggestionsResult message must be posted to the webview");
            assert.strictEqual(result.requestId, "req-1");
            assert.ok(Array.isArray(result.suggestions), "suggestions must be an array");
            assert.strictEqual(result.suggestions.length, 1);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
            vscode.__resetUiState();
            vscode.__resetFindFilesResults();
        }
    });

    test("controller returns workspace-backed suggestions when the query is blank", async () => {
        const { dir, docPath } = makeMentionWorkspace();
        try {
            vscode.__resetUiState();
            vscode.__setFindFilesResults([vscode.Uri.file(path.join(dir, "first.ts"))]);

            const { panel } = openProviderForDoc(dir, docPath);

            panel.fireMessage({
                type: "vector.chatInput.requestSuggestions",
                requestId: "req-empty",
                query: "",
            });

            await new Promise<void>((r) => setTimeout(r, 50));

            const posted = vscode.__getPostedMessages(panel.panel);
            const result = posted.find(
                (m): m is { type: string; requestId: string; suggestions: unknown[] } =>
                    typeof m === "object" &&
                    m !== null &&
                    (m as Record<string, unknown>)["type"] === "vector.chatInput.suggestionsResult",
            ) as { type: string; requestId: string; suggestions: unknown[] } | undefined;
            assert.ok(result, "a suggestionsResult must still be posted");
            assert.strictEqual(result.suggestions.length, 1);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
            vscode.__resetUiState();
            vscode.__resetFindFilesResults();
        }
    });

    test("controller returns empty suggestions when findFiles throws", async () => {
        const { dir, docPath } = makeMentionWorkspace();
        try {
            vscode.__resetUiState();
            vscode.__mockFindFilesThrow(new Error("workspace search failed"));

            const { panel } = openProviderForDoc(dir, docPath);

            panel.fireMessage({
                type: "vector.chatInput.requestSuggestions",
                requestId: "req-fail",
                query: "test",
            });

            await new Promise<void>((r) => setTimeout(r, 50));

            const posted = vscode.__getPostedMessages(panel.panel);
            const result = posted.find(
                (m): m is { type: string; requestId: string; suggestions: unknown[] } =>
                    typeof m === "object" &&
                    m !== null &&
                    (m as Record<string, unknown>)["type"] === "vector.chatInput.suggestionsResult",
            ) as { type: string; requestId: string; suggestions: unknown[] } | undefined;
            assert.ok(result, "a suggestionsResult must be posted even on failure");
            assert.deepStrictEqual(result.suggestions, []);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
            vscode.__resetUiState();
            vscode.__resetFindFilesResults();
        }
    });

    test("resolveFileSuggestions returns workspace results for a blank query", async () => {
        vscode.__setFindFilesResults([vscode.Uri.file(path.join("/workspace", "alpha.ts"))]);
        const result = await resolveFileSuggestions("/workspace", "");
        assert.strictEqual(result.length, 1);
        assert.strictEqual(result[0]?.path, "alpha.ts");
        vscode.__resetFindFilesResults();
    });

    test("resolveFileSuggestions returns workspace results for a whitespace-only query", async () => {
        vscode.__setFindFilesResults([vscode.Uri.file(path.join("/workspace", "beta.ts"))]);
        const result = await resolveFileSuggestions("/workspace", "   ");
        assert.strictEqual(result.length, 1);
        assert.strictEqual(result[0]?.path, "beta.ts");
        vscode.__resetFindFilesResults();
    });

    test("resolveFileSuggestions maps URI results to FileSuggestion objects", async () => {
        const dir = os.tmpdir();
        const filePath = path.join(dir, "formRenderer.ts");
        vscode.__setFindFilesResults([vscode.Uri.file(filePath)]);
        const result = await resolveFileSuggestions(dir, "formRenderer");
        assert.strictEqual(result.length, 1);
        const first = result[0];
        assert.ok(first, "first suggestion must exist");
        assert.strictEqual(first.label, "formRenderer.ts");
        assert.strictEqual(first.path, "formRenderer.ts");
        vscode.__resetFindFilesResults();
    });

    test("resolveFileSuggestions returns empty array when findFiles throws", async () => {
        vscode.__mockFindFilesThrow(new Error("search error"));
        const result = await resolveFileSuggestions("/workspace", "query");
        assert.deepStrictEqual(result, []);
        vscode.__resetFindFilesResults();
    });

    test("runAgent message with chatInputMentions is accepted by the provider", () => {
        const { dir, docPath } = makeMentionWorkspace();
        try {
            vscode.__resetUiState();
            const { panel } = openProviderForDoc(dir, docPath);

            panel.fireMessage({
                type: "vector.runAgent",
                profile: "code",
                prompt: "prompt-001",
                label: "Run",
                staticInput: {},
                formValues: { body: "review @src/formRenderer.ts" },
                chatInputMentions: {
                    body: [
                        {
                            type: "file",
                            label: "formRenderer.ts",
                            path: "src/formRenderer.ts",
                        },
                    ],
                },
            });

            assert.deepStrictEqual(vscode.__getErrorMessages(), [
                "Vector: .vector/agents.yaml not found — add it to use agent triggers.",
            ]);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
            vscode.__resetUiState();
        }
    });
});
