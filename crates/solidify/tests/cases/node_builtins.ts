// Node's built-in modules: fs, path, os (also as `node:…`).

import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync, rmSync, statSync, appendFileSync } from "fs";
import * as path from "node:path";
import os from "os";

const dir = path.join(os.tmpdir(), "solidify-node-builtins");
rmSync(dir, { recursive: true, force: true });
mkdirSync(dir, { recursive: true });
const file = path.join(dir, "notes.txt");
writeFileSync(file, "hello\n");
appendFileSync(file, "world\n");
console.log(existsSync(file), readFileSync(file, "utf8").split("\n").length, statSync(file).size, statSync(dir).isDirectory());
writeFileSync(path.join(dir, "b.txt"), "b");
console.log(readdirSync(dir));
console.log(path.basename(file), path.extname(file), path.dirname("/a/b/c.txt"), path.basename("/a/b/c.txt", ".txt"));
console.log(path.join("a", "b", "../c", "./d"), path.normalize("/a//b/../c/"), path.isAbsolute("/x"), path.relative("/a/b", "/a/c/d"));
console.log(path.parse("/home/u/file.tar.gz"), path.sep, os.EOL === "\n");
try {
  readFileSync(path.join(dir, "missing.txt"), "utf8");
} catch (e) {
  console.log("caught", (e as Error).message.startsWith("ENOENT"));
}
rmSync(dir, { recursive: true });
console.log(existsSync(dir), typeof os.platform(), process.platform === os.platform());
