import * as assert from "assert";
import { renderFormBlock } from "../document-viewer/form-editor/formRenderer.js";
import { renderGovernedMarkdown } from "../document-viewer/markdownRenderer.js";

suite("Phase A — vector-form grid layout", () => {
    test("empty block returns empty string", () => {
        assert.strictEqual(renderFormBlock(""), "");
    });

    test("block with no recognisable fields returns empty string", () => {
        assert.strictEqual(renderFormBlock("not a field\nanother line"), "");
    });

    test("renders outer .vector-form container", () => {
        const html = renderFormBlock(`name = input("Name")`);
        assert.ok(html.includes('class="vector-form"'), "should have vector-form container");
    });

    test("renders one data-bound element per field", () => {
        const html = renderFormBlock(`name = input("Name")\ntitle = input("Title")`);
        const count = (html.match(/data-form-key=/g) ?? []).length;
        assert.strictEqual(count, 2, "should render one data-bound element per field");
    });

    test("editable input field contains .vector-form-label and .vector-form-input", () => {
        const html = renderFormBlock(`name = input("Name")`);
        assert.ok(html.includes('class="vector-form-label"'), "should have vector-form-label");
        assert.ok(html.includes('class="vector-form-input"'), "should have vector-form-input");
    });

    test("label text matches the quoted label from the field definition", () => {
        const html = renderFormBlock(`city = input("City Name")`);
        assert.ok(html.includes("City Name"), "label text should appear in output");
    });

    test("read-only field renders .vector-form-label--readonly and .vector-form-readonly-value", () => {
        const html = renderFormBlock("status = input(active)");
        assert.ok(
            html.includes("vector-form-label--readonly"),
            "should have readonly label modifier class",
        );
        assert.ok(
            html.includes('class="vector-form-readonly-value"'),
            "should have readonly value element",
        );
        assert.ok(html.includes("active"), "should display the substituted value");
    });

    test("read-only field does not render an input element", () => {
        const html = renderFormBlock("status = input(active)");
        assert.ok(!html.includes("<input"), "read-only field must not render an <input>");
        assert.ok(!html.includes("<textarea"), "read-only field must not render a <textarea>");
    });

    test("chat-input field renders the editor host element", () => {
        const html = renderFormBlock(`body = chat-input("Message")`);
        assert.ok(
            html.includes('class="vector-chat-input-host"'),
            "chat-input should render the editor host container",
        );
        assert.ok(
            !html.includes("vector-form-textarea"),
            "chat-input should not use the old vector-form-textarea class",
        );
    });

    test("data-form-key and data-form-type attributes are set on each field control", () => {
        const html = renderFormBlock(`title = input("Title")`);
        assert.ok(
            html.includes('data-form-key="title"'),
            "should set data-form-key to the field key",
        );
        assert.ok(
            html.includes('data-form-type="input"'),
            "should set data-form-type to the field type",
        );
    });

    test("multiple fields produce separate wrappers in document order", () => {
        const html = renderFormBlock(
            `first = input("First")\nsecond = input("Second")\nthird = input("Third")`,
        );
        const firstPos = html.indexOf("First");
        const secondPos = html.indexOf("Second");
        const thirdPos = html.indexOf("Third");
        assert.ok(firstPos < secondPos, "First should appear before Second");
        assert.ok(secondPos < thirdPos, "Second should appear before Third");
    });

    test("vector-form fence block is rendered via the markdown pipeline", () => {
        const source = '```vector-form\nname = input("Name")\n```';
        const html = renderGovernedMarkdown(source);
        assert.ok(html.includes('class="vector-form"'), "markdown pipeline should render the form");
        assert.ok(
            !html.includes("vector-code-block"),
            "vector-form block must not fall back to code block",
        );
    });

    test("unrecognised lines inside a form block are silently ignored", () => {
        const html = renderFormBlock(`# comment\nname = input("Name")\nbad line`);
        const fieldCount = (html.match(/data-form-key=/g) ?? []).length;
        assert.strictEqual(fieldCount, 1, "only the valid field should be rendered");
    });
});

suite("Phase B — Replace textarea with editor host element", () => {
    test("editable chat-input renders .vector-chat-input-host wrapper", () => {
        const html = renderFormBlock(`body = chat-input("Message")`);
        assert.ok(
            html.includes('class="vector-chat-input-host"'),
            "should render the editor host container",
        );
    });

    test("editable chat-input host carries data-chat-input-name attribute", () => {
        const html = renderFormBlock(`body = chat-input("Message")`);
        assert.ok(
            html.includes('data-chat-input-name="body"'),
            "host should carry the field key as data-chat-input-name",
        );
    });

    test("editable chat-input does not render rows=10 on the textarea", () => {
        const html = renderFormBlock(`body = chat-input("Message")`);
        assert.ok(!html.includes('rows="10"'), "editor host must not use a rows=10 textarea");
    });

    test("editable chat-input does not use vector-form-textarea class", () => {
        const html = renderFormBlock(`body = chat-input("Message")`);
        assert.ok(
            !html.includes("vector-form-textarea"),
            "editor host must not use the old vector-form-textarea class",
        );
    });

    test("editable chat-input renders a dedicated editor mount for runtime initialization", () => {
        const html = renderFormBlock(`body = chat-input("Message")`);
        assert.ok(
            !html.includes('contenteditable="true"'),
            "Phase A must stop rendering a contenteditable editor surface",
        );
        assert.ok(
            html.includes('class="vector-chat-input-mount'),
            "the editor mount must carry the vector-chat-input-mount class",
        );
        assert.ok(
            html.includes('data-chat-input-editable="true"'),
            "the editor mount must carry the data-chat-input-editable marker",
        );
        assert.ok(!html.includes("<textarea"), "chat-input must not fall back to a plain textarea");
    });

    test("read-only chat-input does not render the editor host element", () => {
        const html = renderFormBlock("body = chat-input(some value)");
        assert.ok(
            !html.includes("vector-chat-input-host"),
            "read-only chat-input must not render the editor host",
        );
        assert.ok(!html.includes("<textarea"), "read-only chat-input must not render a textarea");
    });

    test("regular input fields are not affected by the chat-input host change", () => {
        const html = renderFormBlock(`title = input("Title")`);
        assert.ok(
            html.includes('<input class="vector-form-input"'),
            "regular input must still use <input> with vector-form-input class",
        );
        assert.ok(
            !html.includes("vector-chat-input-host"),
            "regular input must not render the chat-input host",
        );
    });

    test("chat-input editor host is rendered via the markdown pipeline", () => {
        const source = '```vector-form\nbody = chat-input("Message")\n```';
        const html = renderGovernedMarkdown(source);
        assert.ok(
            html.includes('class="vector-chat-input-host"'),
            "markdown pipeline should render the chat-input host element",
        );
    });
});
