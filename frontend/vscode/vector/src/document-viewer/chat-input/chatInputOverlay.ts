export interface OverlayAnchors {
    cursorX: number;
    cursorBottom: number;
    lineHeight: number;
}

export interface OverlayDimensions {
    width: number;
    height: number;
}

export interface ViewportSize {
    width: number;
    height: number;
}

export interface OverlayPosition {
    top: number;
    left: number;
    flipped: boolean;
}

/**
 * Computes the fixed viewport position for the mention suggestion overlay.
 * Positions the overlay below the cursor and flips it upward when it would
 * overflow the viewport bottom. Clamps the left edge when it would overflow
 * the viewport right edge.
 */
export function computeOverlayPosition(
    anchors: OverlayAnchors,
    dims: OverlayDimensions,
    viewport: ViewportSize,
): OverlayPosition {
    const BELOW_GAP = 2;

    let top = anchors.cursorBottom + BELOW_GAP;
    let flipped = false;

    if (top + dims.height > viewport.height) {
        top = anchors.cursorBottom - anchors.lineHeight - dims.height - BELOW_GAP;
        flipped = true;
    }

    const rawLeft = anchors.cursorX;
    const rightOverflow = rawLeft + dims.width - viewport.width;
    const left = rightOverflow > 0 ? Math.max(0, rawLeft - rightOverflow) : rawLeft;

    return { top, left, flipped };
}
