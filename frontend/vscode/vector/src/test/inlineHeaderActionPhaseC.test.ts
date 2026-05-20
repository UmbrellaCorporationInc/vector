import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";

const extensionRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const mediaDir = path.join(extensionRoot, "media");
const previewJsPath = path.join(mediaDir, "preview.js");
const previewCssPath = path.join(mediaDir, "preview.css");

suite("Task 00042 Phase C — overlay prompt enrichment flow", () => {
    let previewJs: string;
    let previewCss: string;

    suiteSetup(() => {
        previewJs = fs.readFileSync(previewJsPath, "utf-8");
        previewCss = fs.readFileSync(previewCssPath, "utf-8");
    });

    // --- overlay structure ---

    test("preview.js builds an overlay panel with role=dialog and aria-modal", () => {
        assert.ok(
            previewJs.includes('setAttribute("role", "dialog")'),
            "overlay panel must declare role=dialog",
        );
        assert.ok(
            previewJs.includes('setAttribute("aria-modal", "true")'),
            "overlay panel must declare aria-modal=true",
        );
    });

    test("preview.js creates an info control that exposes role=button and tabindex", () => {
        assert.ok(
            previewJs.includes('setAttribute("role", "button")'),
            "info control must carry role=button",
        );
        assert.ok(
            previewJs.includes('setAttribute("tabindex", "0")'),
            "info control must be keyboard reachable via tabindex=0",
        );
    });

    test("preview.js creates a textarea with the correct aria-label for extra content", () => {
        assert.ok(
            previewJs.includes('"Prompt"'),
            "textarea must carry aria-label for extra prompt content",
        );
    });

    test("preview.js creates cancel and submit buttons", () => {
        assert.ok(
            previewJs.includes("vector-inline-overlay-cancel"),
            "overlay must include a cancel button",
        );
        assert.ok(
            previewJs.includes("vector-inline-overlay-submit"),
            "overlay must include a submit button",
        );
    });

    // --- overlay trigger ---

    test("preview.js intercepts vector-agent-inline-action clicks and opens the overlay instead of dispatching immediately", () => {
        assert.ok(
            previewJs.includes('classList.contains("vector-agent-inline-action")'),
            "click handler must branch on vector-agent-inline-action class",
        );
        assert.ok(
            previewJs.includes("openInlineOverlay"),
            "click handler must call openInlineOverlay for inline actions",
        );
    });

    test("preview.js does not call postMessage for inline actions before the overlay is submitted", () => {
        const inlineBlock = previewJs.slice(
            previewJs.indexOf('classList.contains("vector-agent-inline-action")'),
            previewJs.indexOf(
                "return;",
                previewJs.indexOf('classList.contains("vector-agent-inline-action")'),
            ),
        );
        assert.ok(
            !inlineBlock.includes("postMessage"),
            "inline action intercept block must not directly call postMessage",
        );
    });

    // --- prompt-message merge ---

    test("preview.js includes prompt-message as the key for extra input", () => {
        assert.ok(
            previewJs.includes('"prompt-message"'),
            "submit path must assign prompt-message to the static input",
        );
    });

    test("preview.js trims the textarea value before deciding to include prompt-message", () => {
        assert.ok(
            previewJs.includes(".trim()"),
            "submit path must trim extra input before including it",
        );
    });

    test("preview.js only adds prompt-message when the trimmed value is non-empty", () => {
        assert.ok(
            previewJs.includes("if (extra)") || previewJs.includes("if(extra)"),
            "prompt-message must be conditionally added based on trimmed content",
        );
    });

    // --- cancel and close ---

    test("preview.js wires cancelBtn click to closeInlineOverlay", () => {
        assert.ok(
            previewJs.includes("closeInlineOverlay"),
            "cancel and close paths must call closeInlineOverlay",
        );
    });

    test("preview.js closes the overlay and clears pending action on close", () => {
        const closeFn = previewJs.slice(
            previewJs.indexOf("function closeInlineOverlay"),
            previewJs.indexOf("}", previewJs.indexOf("function closeInlineOverlay") + 1),
        );
        assert.ok(
            closeFn.includes("overlayPendingAction = null"),
            "closeInlineOverlay must clear the pending action",
        );
        assert.ok(
            closeFn.includes('classList.remove("is-open")'),
            "closeInlineOverlay must remove is-open from the overlay container",
        );
    });

    // --- keyboard behavior ---

    test("preview.js handles Escape key to close the overlay", () => {
        assert.ok(previewJs.includes('"Escape"'), "keydown handler must handle the Escape key");
    });

    test("preview.js implements Tab focus trapping within the overlay", () => {
        assert.ok(
            previewJs.includes('"Tab"'),
            "keydown handler must handle Tab for focus trapping",
        );
        assert.ok(previewJs.includes("focusables"), "focus trap must use a focusables list");
    });

    test("preview.js supports Shift+Tab for reverse focus navigation", () => {
        assert.ok(
            previewJs.includes("e.shiftKey"),
            "Tab handler must check shiftKey for reverse navigation",
        );
    });

    test("preview.js activates the info control on Enter and Space", () => {
        assert.ok(
            previewJs.includes('"Enter"') && previewJs.includes('" "'),
            "info control keydown handler must respond to Enter and Space",
        );
    });

    // --- backdrop dismiss ---

    test("preview.js closes the overlay when the backdrop is clicked", () => {
        assert.ok(
            previewJs.includes("backdrop.addEventListener") &&
                previewJs.includes("closeInlineOverlay"),
            "backdrop click listener must call closeInlineOverlay",
        );
        const backdropIdx = previewJs.indexOf("backdrop.addEventListener");
        const closeAfterBackdrop = previewJs.indexOf("closeInlineOverlay", backdropIdx);
        assert.ok(
            closeAfterBackdrop !== -1,
            "closeInlineOverlay must appear after backdrop.addEventListener registration",
        );
    });

    // --- info control also triggers execution ---

    test("preview.js registers a click handler on the info control that also submits", () => {
        assert.ok(
            previewJs.includes("infoControl.addEventListener"),
            "info control must have event listeners wired to the submit path",
        );
    });

    test("preview.js opens the overlay via openInlineOverlay with correct action fields", () => {
        assert.ok(
            previewJs.includes("openInlineOverlay({"),
            "openInlineOverlay must be called with an action object",
        );
        assert.ok(
            previewJs.includes("profile: profile") && previewJs.includes("prompt: prompt"),
            "action object must carry profile and prompt fields",
        );
    });

    // --- CSS overlay structure ---

    test("preview.css defines the overlay container hidden by default and shown via is-open", () => {
        assert.ok(
            previewCss.includes(".vector-inline-overlay {"),
            "CSS must define .vector-inline-overlay",
        );
        assert.ok(previewCss.includes("display: none"), "overlay must be hidden by default");
        assert.ok(
            previewCss.includes(".vector-inline-overlay.is-open"),
            "overlay must become visible via .is-open class",
        );
        assert.ok(
            previewCss.includes("display: flex") &&
                previewCss.includes(".vector-inline-overlay.is-open"),
            "is-open must switch the overlay to flex layout",
        );
    });

    test("preview.css includes a backdrop layer with a dimming background", () => {
        assert.ok(
            previewCss.includes(".vector-inline-overlay-backdrop"),
            "CSS must define the backdrop element",
        );
        assert.ok(
            previewCss.includes("position: absolute") && previewCss.includes("inset: 0"),
            "backdrop must cover the full viewport",
        );
    });

    test("preview.css defines the overlay panel with border-radius and box-shadow", () => {
        assert.ok(
            previewCss.includes(".vector-inline-overlay-panel"),
            "CSS must define the overlay panel",
        );
        assert.ok(
            previewCss.includes("border-radius") && previewCss.includes("box-shadow"),
            "panel must use border-radius and box-shadow for visual elevation",
        );
    });

    test("preview.css defines focus-visible styles for cancel and submit buttons", () => {
        assert.ok(
            previewCss.includes(".vector-inline-overlay-cancel:focus-visible"),
            "cancel button must have a focus-visible style",
        );
        assert.ok(
            previewCss.includes(".vector-inline-overlay-submit:focus-visible"),
            "submit button must have a focus-visible style",
        );
    });

    test("preview.css defines hover styles for submit and cancel buttons", () => {
        assert.ok(
            previewCss.includes(".vector-inline-overlay-submit:hover"),
            "submit button must have a hover style",
        );
        assert.ok(
            previewCss.includes(".vector-inline-overlay-cancel:hover"),
            "cancel button must have a hover style",
        );
    });
});
