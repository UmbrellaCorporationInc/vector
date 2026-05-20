import * as path from "path";
import { fileURLToPath } from "url";
import Mocha from "mocha";
import { glob } from "glob";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export async function run(): Promise<void> {
    const mocha = new Mocha({ ui: "tdd", color: true, timeout: 10000 });
    const tests_root = path.resolve(__dirname);
    const files = await glob("**/*.test.js", { cwd: tests_root });

    // Register Mocha globals (suite, test, etc.) before loading ESM test files
    mocha.suite.emit("pre-require", global, "", mocha);

    for (const file of files) {
        mocha.addFile(path.resolve(tests_root, file));
    }

    await mocha.loadFilesAsync();

    return new Promise<void>((resolve, reject) => {
        mocha.run((failures) => {
            if (failures > 0) {
                reject(new Error(`${String(failures)} test(s) failed.`));
            } else {
                resolve();
            }
        });
    });
}

run().catch((err: unknown) => {
    console.error(err);
    process.exit(1);
});
