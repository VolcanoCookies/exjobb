import { existsSync, mkdirSync, readdirSync, copyFileSync, statSync } from "fs";
import { join } from "path";

// @ts-ignore
function copyFiles(src, dest) {
    if (!existsSync(dest)) {
        mkdirSync(dest);
    }

    const files = readdirSync(src);

    files.forEach((file) => {
        if (file.endsWith(".ts")) return;

        const srcPath = join(src, file);
        const destPath = join(dest, file);

        if (existsSync(srcPath)) {
            const stat = statSync(srcPath);

            if (stat.isDirectory()) {
                copyFiles(srcPath, destPath);
            } else if (stat.isFile()) {
                copyFileSync(srcPath, destPath);
            }
        }
    });
}

const src = "./src";
const dest = "./dist";

copyFiles(src, dest);