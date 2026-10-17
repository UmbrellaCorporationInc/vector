import * as path from "path";
import { normalizeDocPath } from "./actionsConfig.js";

/** Returns source metadata relative to a workspace or synchronized package doc directory. */
export function documentActionMetadata(
    workspaceRoot: string,
    documentPath: string,
): Record<string, string> | null {
    const relative = path.relative(path.resolve(workspaceRoot), path.resolve(documentPath));
    if (
        !relative ||
        relative === ".." ||
        relative.startsWith(`..${path.sep}`) ||
        path.isAbsolute(relative)
    ) {
        return null;
    }
    const segments = relative.split(path.sep);
    const docSegments =
        segments[0] === "doc"
            ? segments.slice(1)
            : segments[0] === ".vector-database" &&
                segments[1] === "packages" &&
                segments[2] &&
                segments[3] === "doc"
              ? segments.slice(4)
              : [];
    if (docSegments.length === 0) {
        return null;
    }
    return {
        "file-path": normalizeDocPath(docSegments.join(path.sep)),
        "file-name": path.basename(documentPath),
    };
}
