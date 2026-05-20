import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";
import { renderFormBlock } from "../document-viewer/form-editor/formRenderer.js";

const extensionRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const sourceRoot = path.join(extensionRoot, "src");
const mediaDir = path.join(extensionRoot, "media");

function listFiles(dir: string, suffix: string): string[] {
    const results: string[] = [];
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const fullPath = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            results.push(...listFiles(fullPath, suffix));
            continue;
        }
        if (entry.isFile() && fullPath.endsWith(suffix)) {
            results.push(fullPath);
        }
    }
    return results;
}

suite("Task 00037 Phase F - folder rename fallout", () => {
    test("source files no longer reference underscored viewer module folders", () => {
        const sourceFiles = listFiles(sourceRoot, ".ts");
        for (const filePath of sourceFiles) {
            if (filePath.endsWith(path.join("src", "test", "chatInputPhaseF.test.ts"))) {
                continue;
            }
            const content = fs.readFileSync(filePath, "utf-8");
            assert.ok(
                !content.includes("form_editor"),
                `source file must not reference form_editor: ${path.relative(extensionRoot, filePath)}`,
            );
            assert.ok(
                !content.includes("document_actions"),
                "source file must not reference document_actions: " +
                    path.relative(extensionRoot, filePath),
            );
        }
    });

    test("public document-viewer exports use the kebab-case module paths", () => {
        const indexPath = path.join(sourceRoot, "document-viewer", "index.ts");
        const content = fs.readFileSync(indexPath, "utf-8");
        assert.ok(content.includes("./form-editor/formParser.js"));
        assert.ok(content.includes("./form-editor/formRenderer.js"));
        assert.ok(content.includes("./document-actions/variableSubstitution.js"));
        assert.ok(content.includes("./document-actions/agentExecutor.js"));
    });
});

suite("Task 00037 Phase F - chat-input rendering modes", () => {
    test("read-only chat-input renders a read-only value surface instead of the editor host", () => {
        const html = renderFormBlock("body = chat-input(existing value)");
        assert.ok(html.includes('class="vector-form-readonly-value"'));
        assert.ok(html.includes(">existing value<"));
        assert.ok(!html.includes("vector-chat-input-host"));
        assert.ok(!html.includes('contenteditable="true"'));
    });
});

suite("Task 00037 Phase F - webview runtime contract", () => {
    test("preview.js posts both plain form values and chat-input mentions to runAgent", () => {
        const previewScript = fs.readFileSync(path.join(mediaDir, "preview.js"), "utf-8");
        assert.ok(previewScript.includes('type: "vector.runAgent"'));
        assert.ok(previewScript.includes("formValues: formValues"));
        assert.ok(previewScript.includes("chatInputMentions: chatInputMentions"));
    });

    test("preview.js delegates suggestion handling and collection to the dedicated runtime module", () => {
        const previewScript = fs.readFileSync(path.join(mediaDir, "preview.js"), "utf-8");
        assert.ok(
            previewScript.includes("window.VectorChatInputRuntime.create({ vscode: vscode })"),
        );
        assert.ok(previewScript.includes("chatInputRuntime.handleSuggestionsResult(msg);"));
        assert.ok(previewScript.includes("chatInputRuntime.collectFormValues()"));
        assert.ok(previewScript.includes("chatInputRuntime.collectMentions()"));
    });
});

suite("Task 00037 Phase F - first iteration runtime ignores mentions", () => {
    test("handleRunAgent merges only staticInput and plain formValues into the prompt variables", () => {
        const providerSource = fs.readFileSync(
            path.join(sourceRoot, "document-viewer", "governedDocumentEditorProvider.ts"),
            "utf-8",
        );
        assert.ok(
            providerSource.includes("chatInputMentions?: Record<string, ChatInputMention[]>;"),
        );
        assert.ok(
            providerSource.includes(
                "const mergedVars: Record<string, string> = { ...msg.staticInput, ...msg.formValues };",
            ),
            "runtime must resolve prompts from plain text fields only in the first iteration",
        );
        assert.ok(
            !providerSource.includes("...msg.chatInputMentions"),
            "mentions metadata must not be merged into the runtime prompt variables",
        );
    });
});

suite("Task 00037 Phase F - markdown styling and dynamic height contracts", () => {
    test("chat-input-runtime.js defines the editor surface through a CodeMirror theme", () => {
        const runtimeSource = fs.readFileSync(
            path.join(mediaDir, "chat-input-runtime.js"),
            "utf-8",
        );
        assert.ok(runtimeSource.includes("var chatInputTheme = EditorView.theme({"));
        assert.ok(runtimeSource.includes('minHeight: "4rem"'));
        assert.ok(runtimeSource.includes('maxHeight: "20rem"'));
        assert.ok(runtimeSource.includes('overflowY: "auto"'));
        assert.ok(runtimeSource.includes('whiteSpace: "pre-wrap"'));
    });

    test("preview.css keeps the vector-form grid layout responsive as the editor grows", () => {
        const previewCss = fs.readFileSync(path.join(mediaDir, "preview.css"), "utf-8");
        assert.ok(previewCss.includes(".vector-form {"));
        assert.ok(previewCss.includes("grid-template-columns: max-content minmax(0, 1fr);"));
        assert.ok(previewCss.includes("column-gap: 1rem;"));
        assert.ok(previewCss.includes("@media (max-width: 480px) {"));
        assert.ok(previewCss.includes("grid-template-columns: 1fr;"));
    });

    test("the dedicated runtime module owns CodeMirror editor state and plain-text collection", () => {
        const runtimeScript = fs.readFileSync(
            path.join(mediaDir, "chat-input-runtime.js"),
            "utf-8",
        );
        assert.ok(runtimeScript.includes('from "@codemirror/state"'));
        assert.ok(runtimeScript.includes('from "@codemirror/view"'));
        assert.ok(runtimeScript.includes('from "@codemirror/autocomplete"'));
        assert.ok(runtimeScript.includes("autocompletion({"));
        assert.ok(runtimeScript.includes("startCompletion(instance.view)"));
        assert.ok(runtimeScript.includes("new EditorView({ state: state, parent: mountEl })"));
        assert.ok(runtimeScript.includes("instance.view.state.doc.toString()"));
        assert.ok(runtimeScript.includes("window.VectorChatInputRuntime = {"));
    });
});
