import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";
import {
    renderGovernedMarkdown,
    renderGovernedMarkdownAnalysis,
} from "../document-viewer/markdownRenderer.js";
import { renderInlineHeaderAction } from "../document-viewer/document-actions/inlineHeaderActionRenderer.js";
import { renderAgentBlock } from "../document-viewer/document-actions/agentBlockRenderer.js";
import { loadDocumentTypes } from "../documentDiscovery.js";
import * as os from "os";

const extensionRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const mediaDir = path.join(extensionRoot, "media");
const previewCss = fs.readFileSync(path.join(mediaDir, "preview.css"), "utf-8");
const previewJs = fs.readFileSync(path.join(mediaDir, "preview.js"), "utf-8");
const extensionSrc = fs.readFileSync(path.join(extensionRoot, "src", "extension.ts"), "utf-8");
const providerSrc = fs.readFileSync(
    path.join(extensionRoot, "src", "document-viewer", "governedDocumentEditorProvider.ts"),
    "utf-8",
);
const pkg = JSON.parse(fs.readFileSync(path.join(extensionRoot, "package.json"), "utf-8")) as {
    contributes: {
        commands: Array<{ command: string; title: string; icon?: string }>;
        menus: Record<string, Array<{ command: string; when: string; group: string }>>;
    };
};

function makeTempWorkspace(config: string): string {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-phase-e-test-"));
    const vectorDir = path.join(dir, ".vector");
    fs.mkdirSync(vectorDir, { recursive: true });
    fs.writeFileSync(path.join(vectorDir, "document-types.yaml"), config, "utf-8");
    return dir;
}

// ---------------------------------------------------------------------------
// RFC 00023 AC-1 — vector-agent-inline-action component
// ---------------------------------------------------------------------------

suite("Task 00042 Phase E — RFC 00023 AC: inline action component", () => {
    test("renderInlineHeaderAction produces a button with vector-agent-inline-action and vector-agent-header-action classes", () => {
        const html = renderInlineHeaderAction("task-00042-foo");
        assert.ok(
            html.includes("vector-agent-inline-action"),
            "button must carry vector-agent-inline-action class for JS overlay detection",
        );
        assert.ok(
            html.includes("vector-agent-header-action"),
            "button must carry vector-agent-header-action class for pencil styling",
        );
    });

    test("renderGovernedMarkdownAnalysis injects inline action into every heading", () => {
        const source = "# H1\n## H2\n### H3";
        const html = renderGovernedMarkdownAnalysis(source, {
            documentStem: "task-00042-foo",
        }).html;
        const count = (html.match(/vector-agent-inline-action/g) ?? []).length;
        assert.strictEqual(count, 3, "one inline action per heading");
    });

    test("renderGovernedMarkdown without documentStem produces no inline actions", () => {
        const html = renderGovernedMarkdown("# Heading\n## Sub");
        assert.ok(
            !html.includes("vector-agent-inline-action"),
            "legacy render path must not inject inline actions",
        );
    });
});

// ---------------------------------------------------------------------------
// RFC 00023 AC-2/3 — overlay opens before execution; prompt-message merge
// ---------------------------------------------------------------------------

suite("Task 00042 Phase E — RFC 00023 AC: overlay execution path", () => {
    test("preview.js intercepts inline action clicks and routes to openInlineOverlay", () => {
        assert.ok(
            previewJs.includes('classList.contains("vector-agent-inline-action")'),
            "click handler must detect the inline action class",
        );
        assert.ok(previewJs.includes("openInlineOverlay"), "click must route to openInlineOverlay");
    });

    test("overlay merges submitted text as prompt-message before posting runAgent", () => {
        assert.ok(
            previewJs.includes('"prompt-message"'),
            "submit path must assign extra input to prompt-message",
        );
    });

    test("overlay trims and conditionally includes prompt-message", () => {
        assert.ok(previewJs.includes(".trim()"), "submit path must trim extra input");
        assert.ok(
            previewJs.includes("if (extra)") || previewJs.includes("if(extra)"),
            "prompt-message is only added when the trimmed value is non-empty",
        );
    });
});

// ---------------------------------------------------------------------------
// RFC 00023 AC-4 — info control inside the overlay
// ---------------------------------------------------------------------------

suite("Task 00042 Phase E — RFC 00023 AC: overlay info control", () => {
    test("overlay contains an info control with role=button", () => {
        assert.ok(
            previewJs.includes('setAttribute("role", "button")'),
            "info control must declare role=button",
        );
    });

    test("info control is keyboard reachable", () => {
        assert.ok(
            previewJs.includes('setAttribute("tabindex", "0")'),
            "info control must have tabindex=0",
        );
    });

    test("info control click wires to the submit path", () => {
        assert.ok(
            previewJs.includes("infoControl.addEventListener"),
            "info control must register an event listener for submission",
        );
    });
});

// ---------------------------------------------------------------------------
// RFC 00023 AC-5/6 — header action contract
// ---------------------------------------------------------------------------

suite("Task 00042 Phase E — RFC 00023 AC: header action contract", () => {
    test("header inline action is bound to prompts-00006-update-document", () => {
        const html = renderInlineHeaderAction("task-00042-foo");
        assert.ok(
            html.includes('data-agent-prompt="prompts-00006-update-document"'),
            "inline action must bind to prompts-00006-update-document",
        );
    });

    test("header inline action passes profile=create-doc", () => {
        const html = renderInlineHeaderAction("task-00042-foo");
        assert.ok(
            html.includes('data-agent-profile="create-doc"'),
            "inline action must use the create-doc profile",
        );
    });

    test("header inline action encodes document-stem in the input payload", () => {
        const stem = "task-00042-implement-rfc-00023";
        const html = renderInlineHeaderAction(stem);
        assert.ok(html.includes(stem), "document-stem must appear in the input payload");
    });

    test("rendered heading action carries document-header matching heading text", () => {
        const html = renderGovernedMarkdownAnalysis("## My Section", {
            documentStem: "task-00042-foo",
        }).html;
        assert.ok(html.includes("document-header"), "input must include document-header");
        assert.ok(html.includes("My Section"), "document-header value must match heading text");
    });
});

// ---------------------------------------------------------------------------
// RFC 00023 AC-7 — pencil affordance
// ---------------------------------------------------------------------------

suite("Task 00042 Phase E — RFC 00023 AC: pencil affordance", () => {
    test("inline action button displays the pencil glyph ✏", () => {
        const html = renderInlineHeaderAction("task-00042-foo");
        assert.ok(html.includes("✏"), "button must contain the pencil glyph");
    });

    test("inline action carries an aria-label for accessibility", () => {
        const html = renderInlineHeaderAction("task-00042-foo");
        assert.ok(html.includes("aria-label="), "button must carry aria-label");
    });
});

// ---------------------------------------------------------------------------
// RFC 00023 AC-8 — vector-agent-action visible affordances
// ---------------------------------------------------------------------------

suite("Task 00042 Phase E — RFC 00023 AC: vector-agent-action styling", () => {
    test("preview.css defines a default appearance for vector-agent-action", () => {
        assert.ok(
            previewCss.includes(".vector-agent-action"),
            "CSS must define .vector-agent-action",
        );
    });

    test("preview.css defines a hover state for vector-agent-action", () => {
        assert.ok(
            previewCss.includes(".vector-agent-action:hover"),
            "vector-agent-action must have a hover style",
        );
    });

    test("preview.css defines a focus-visible state for vector-agent-action", () => {
        assert.ok(
            previewCss.includes(".vector-agent-action:focus-visible") ||
                previewCss.includes(".vector-agent-action:focus"),
            "vector-agent-action must have a focus style",
        );
    });

    test("preview.css defines a hover state for vector-agent-button", () => {
        assert.ok(
            previewCss.includes(".vector-agent-button:hover"),
            "vector-agent-button must have a hover style",
        );
    });

    test("preview.css defines a focus state for vector-agent-button", () => {
        assert.ok(
            previewCss.includes(".vector-agent-button:focus-visible") ||
                previewCss.includes(".vector-agent-button:focus"),
            "vector-agent-button must have a focus style",
        );
    });

    test("vector-agent-action and vector-agent-button share base layout styles", () => {
        const baseRuleIdx = previewCss.indexOf(".vector-agent-action,");
        assert.ok(
            baseRuleIdx !== -1,
            "a combined base rule for both agent action and button must exist",
        );
        const ruleBlock = previewCss.slice(baseRuleIdx, previewCss.indexOf("}", baseRuleIdx) + 1);
        assert.ok(
            ruleBlock.includes(".vector-agent-button"),
            "base rule must cover vector-agent-button as well",
        );
    });
});

// ---------------------------------------------------------------------------
// RFC 00023 AC-9/10 — container validate-fix action
// ---------------------------------------------------------------------------

suite("Task 00042 Phase E — RFC 00023 AC: container validate-fix action", () => {
    test("extension.ts registers the vector.validateFix command", () => {
        assert.ok(
            extensionSrc.includes('"vector.validateFix"'),
            "extension.ts must register vector.validateFix",
        );
    });

    test("vector.validateFix command is listed in package.json contributes", () => {
        const cmd = pkg.contributes.commands.find((c) => c.command === "vector.validateFix");
        assert.ok(cmd, "vector.validateFix must be declared in package.json");
    });

    test("vector.validateFix is wired as a global view/title action for vector.governedDocuments", () => {
        const menus = pkg.contributes.menus["view/title"] ?? [];
        const entry = menus.find((m) => m.command === "vector.validateFix");
        assert.ok(entry, "view/title entry for vector.validateFix must exist");
        assert.ok(
            entry.when.includes("view == vector.governedDocuments"),
            "entry must target vector.governedDocuments view",
        );
        assert.ok(
            !entry.when.includes("viewItem"),
            "validate-fix must be a global container action, not item-scoped",
        );
    });
});

// ---------------------------------------------------------------------------
// RFC 00023 AC-10 — validate-fix uses create-doc profile
// ---------------------------------------------------------------------------

suite("Task 00042 Phase E — RFC 00023 AC: validate-fix invokes create-doc profile", () => {
    test("extension.ts validate-fix handler reads prompt-validate-fix and opens via vscode.openWith", () => {
        assert.ok(
            extensionSrc.includes("prompt-validate-fix"),
            "extension must read the prompt-validate-fix key from the config",
        );
        const idx = extensionSrc.indexOf('"vector.validateFix"');
        assert.ok(idx !== -1, "vector.validateFix must be registered in extension.ts");
        const block = extensionSrc.slice(idx, idx + 1200);
        assert.ok(
            block.includes('"vscode.openWith"'),
            "extension validate-fix handler must dispatch vscode.openWith (migrated in Task 00044 Phase C)",
        );
    });

    test("GovernedDocumentEditorProvider handles runAgent with correct profile", () => {
        assert.ok(
            providerSrc.includes(
                "private async _handleRunAgent(msg: RunAgentMessage): Promise<void>",
            ),
            "_handleRunAgent must exist in the provider",
        );
    });
});

// ---------------------------------------------------------------------------
// RFC 00023 AC-11/12 — prompt resolved from doc-type.prompt-validate-fix
// ---------------------------------------------------------------------------

suite("Task 00042 Phase E — RFC 00023 AC: prompt resolution from document-types.yaml", () => {
    test("doc-type.prompt-validate-fix is read from the global doc-type block", () => {
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

    test("validate-fix prompt is global: it does not appear inside individual document-types entries", () => {
        const config = `document-types:
  task:
    layout: status
    code-width: 5
    statuses: [todo, in-progress, done]
doc-type:
  prompt-validate-fix: prompts-00007-validate-fix
`;
        const dir = makeTempWorkspace(config);
        try {
            const result = loadDocumentTypes(dir);
            assert.ok(result, "loadDocumentTypes must succeed");
            const taskEntry = result["document-types"]["task"] as unknown as Record<
                string,
                unknown
            >;
            assert.ok(
                !("prompt-validate-fix" in taskEntry),
                "prompt-validate-fix must live in the global doc-type block, not per document-type",
            );
            assert.strictEqual(
                result["doc-type"]?.["prompt-validate-fix"],
                "prompts-00007-validate-fix",
                "prompt-validate-fix must be accessible from the global doc-type section",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});

// ---------------------------------------------------------------------------
// Regression — existing viewer actions and tree controls are unaffected
// ---------------------------------------------------------------------------

suite("Task 00042 Phase E — regression: existing controls are unaffected", () => {
    test("vector-agent-action block still renders with the correct data attributes", () => {
        const yaml = [
            "label: Run Agent",
            "profile: code",
            "prompt: prompts-00001-test",
            "input:",
            "  task: task-00042",
        ].join("\n");
        const html = renderAgentBlock(yaml, "action");
        assert.ok(html.includes('data-agent-profile="code"'), "profile attribute must be present");
        assert.ok(
            html.includes('data-agent-prompt="prompts-00001-test"'),
            "prompt attribute must be present",
        );
        assert.ok(
            html.includes('class="vector-agent-action"'),
            "class must be vector-agent-action",
        );
    });

    test("vector-agent-button block still renders with the correct data attributes", () => {
        const yaml = "label: Run\nprofile: code\nprompt: prompts-00002-test\n";
        const html = renderAgentBlock(yaml, "button");
        assert.ok(
            html.includes('class="vector-agent-button"'),
            "class must be vector-agent-button",
        );
    });

    test("existing package.json commands for search, refresh, and collapse are still present", () => {
        const commands = pkg.contributes.commands.map((c) => c.command);
        assert.ok(commands.includes("vector.searchInType"), "search command must still exist");
        assert.ok(
            commands.includes("vector.refreshGovernedDocuments"),
            "refresh command must still exist",
        );
        assert.ok(
            commands.includes("vector.clearAllFilters"),
            "clearAllFilters command must still exist",
        );
        assert.ok(
            commands.includes("vector.createDocument"),
            "createDocument command must still exist",
        );
        assert.ok(
            commands.includes("vector.createDocumentType"),
            "createDocumentType command must still exist",
        );
    });

    test("view/title still contains search and refresh actions", () => {
        const menus = pkg.contributes.menus["view/title"] ?? [];
        const commands = menus.map((m) => m.command);
        assert.ok(commands.includes("vector.searchInType"), "search must still be in view/title");
        assert.ok(
            commands.includes("vector.refreshGovernedDocuments"),
            "refresh must still be in view/title",
        );
    });

    test("inline header action does not interfere with vector-form fences", () => {
        const source = ["## Section", "", "```vector-form", `body = input("Body")`, "```"].join(
            "\n",
        );
        const html = renderGovernedMarkdownAnalysis(source, {
            documentStem: "task-00042-foo",
        }).html;
        assert.ok(html.includes("vector-form"), "form block must still render");
        assert.ok(
            html.includes("vector-agent-inline-action"),
            "inline action must appear on heading alongside form",
        );
    });

    test("inline action in a document with no headings does not produce stray buttons", () => {
        const source = "Just a paragraph with no headings.";
        const html = renderGovernedMarkdownAnalysis(source, {
            documentStem: "task-00042-foo",
        }).html;
        assert.ok(
            !html.includes("vector-agent-inline-action"),
            "no inline actions must appear when there are no headings",
        );
    });
});
