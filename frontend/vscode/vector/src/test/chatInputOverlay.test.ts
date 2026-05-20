import * as assert from "assert";
import { computeOverlayPosition } from "../document-viewer/chat-input/chatInputOverlay.js";
import type {
    OverlayAnchors,
    OverlayDimensions,
    ViewportSize,
} from "../document-viewer/chat-input/chatInputOverlay.js";

const VIEWPORT: ViewportSize = { width: 800, height: 600 };
const DIMS: OverlayDimensions = { width: 260, height: 160 };

suite("Phase C.5 — computeOverlayPosition: below-cursor placement", () => {
    test("positions below cursor when it fits in the viewport", () => {
        const anchors: OverlayAnchors = { cursorX: 100, cursorBottom: 300, lineHeight: 20 };
        const pos = computeOverlayPosition(anchors, DIMS, VIEWPORT);
        assert.strictEqual(pos.top, 302);
        assert.strictEqual(pos.left, 100);
        assert.strictEqual(pos.flipped, false);
    });

    test("places top exactly BELOW_GAP pixels below cursorBottom", () => {
        const anchors: OverlayAnchors = { cursorX: 50, cursorBottom: 200, lineHeight: 18 };
        const pos = computeOverlayPosition(anchors, DIMS, VIEWPORT);
        assert.strictEqual(pos.top, 202);
    });
});

suite("Phase C.5 — computeOverlayPosition: upward flip on viewport overflow", () => {
    test("flips upward when dropdown would overflow the bottom edge", () => {
        const anchors: OverlayAnchors = { cursorX: 100, cursorBottom: 500, lineHeight: 20 };
        const pos = computeOverlayPosition(anchors, DIMS, VIEWPORT);
        assert.strictEqual(pos.flipped, true);
        assert.ok(pos.top + DIMS.height <= anchors.cursorBottom);
    });

    test("flipped top is cursorBottom minus lineHeight minus height minus gap", () => {
        const anchors: OverlayAnchors = { cursorX: 0, cursorBottom: 500, lineHeight: 20 };
        const pos = computeOverlayPosition(anchors, DIMS, VIEWPORT);
        assert.strictEqual(pos.top, 500 - 20 - 160 - 2);
    });

    test("does not flip when dropdown fits exactly at the bottom boundary", () => {
        const cursorBottom = VIEWPORT.height - DIMS.height - 2;
        const anchors: OverlayAnchors = { cursorX: 0, cursorBottom, lineHeight: 20 };
        const pos = computeOverlayPosition(anchors, DIMS, VIEWPORT);
        assert.strictEqual(pos.flipped, false);
    });

    test("flips when one pixel past the boundary", () => {
        const cursorBottom = VIEWPORT.height - DIMS.height - 1;
        const anchors: OverlayAnchors = { cursorX: 0, cursorBottom, lineHeight: 20 };
        const pos = computeOverlayPosition(anchors, DIMS, VIEWPORT);
        assert.strictEqual(pos.flipped, true);
    });
});

suite("Phase C.5 — computeOverlayPosition: right-edge clamping", () => {
    test("clamps left when dropdown would overflow the right viewport edge", () => {
        const anchors: OverlayAnchors = { cursorX: 620, cursorBottom: 100, lineHeight: 20 };
        const pos = computeOverlayPosition(anchors, DIMS, VIEWPORT);
        assert.strictEqual(pos.left, 800 - 260);
    });

    test("does not clamp when dropdown fits within the right edge", () => {
        const anchors: OverlayAnchors = { cursorX: 400, cursorBottom: 100, lineHeight: 20 };
        const pos = computeOverlayPosition(anchors, DIMS, VIEWPORT);
        assert.strictEqual(pos.left, 400);
    });

    test("clamps to 0 when cursor is near the left edge and overflow exceeds cursor position", () => {
        const anchors: OverlayAnchors = { cursorX: 10, cursorBottom: 100, lineHeight: 20 };
        const wideDims: OverlayDimensions = { width: 900, height: 160 };
        const pos = computeOverlayPosition(anchors, wideDims, VIEWPORT);
        assert.strictEqual(pos.left, 0);
    });

    test("cursor exactly at right edge: left is clamped to viewport width minus dropdown width", () => {
        const anchors: OverlayAnchors = { cursorX: 800, cursorBottom: 100, lineHeight: 20 };
        const pos = computeOverlayPosition(anchors, DIMS, VIEWPORT);
        assert.strictEqual(pos.left, 540);
    });
});
