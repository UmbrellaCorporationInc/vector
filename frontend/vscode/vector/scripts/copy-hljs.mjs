import { copyFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const cdnRoot = join(__dirname, "..", "node_modules", "@highlightjs", "cdn-assets");
const mediaDir = join(__dirname, "..", "media");

copyFileSync(join(cdnRoot, "highlight.min.js"), join(mediaDir, "hljs.min.js"));
copyFileSync(join(cdnRoot, "styles", "vs2015.min.css"), join(mediaDir, "hljs-theme.css"));
