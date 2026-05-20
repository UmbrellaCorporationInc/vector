import js from "@eslint/js";
import tseslint from "typescript-eslint";
import { fileURLToPath } from "url";
import path from "path";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default tseslint.config(
    {
        ignores: ["out/", "node_modules/", ".vscode-test/", "media/"],
    },
    js.configs.recommended,
    tseslint.configs.recommendedTypeChecked,
    tseslint.configs.strictTypeChecked,
    {
        languageOptions: {
            parserOptions: {
                projectService: {
                    allowDefaultProject: ["*.js", "*.mjs", "scripts/*.mjs"],
                },
                tsconfigRootDir: __dirname,
            },
        },
    },
    {
        files: ["eslint.config.js"],
        rules: {
            "@typescript-eslint/no-deprecated": "off",
        },
    },
    {
        files: ["src/test/**/*.ts", "**/*.test.ts"],
        rules: {
            "@typescript-eslint/no-unsafe-argument": "off",
            "@typescript-eslint/no-unsafe-assignment": "off",
            "@typescript-eslint/no-unsafe-member-access": "off",
            "@typescript-eslint/no-unsafe-call": "off",
            "@typescript-eslint/no-unsafe-return": "off",
            "@typescript-eslint/unbound-method": "off",
        },
    },
);
