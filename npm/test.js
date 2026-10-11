// Checks the npm package against the test suite: every program of
// crates/solidify/tests/cases translated by the WebAssembly build must equal the `.rs`
// the native build wrote. Run after copying solidify.wasm here: `node npm/test.js`.
"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const cases = path.join(__dirname, "..", "crates", "solidify", "tests", "cases");
const bin = path.join(__dirname, "bin", "solidify.js");
const programs = [];
for (const entry of fs.readdirSync(cases, { withFileTypes: true })) {
  if (entry.isFile() && entry.name.endsWith(".ts")) {
    programs.push([path.join(cases, entry.name), path.join(cases, entry.name.replace(/\.ts$/, ".rs"))]);
  } else if (entry.isDirectory() && fs.existsSync(path.join(cases, entry.name, "main.ts"))) {
    programs.push([path.join(cases, entry.name, "main.ts"), path.join(cases, entry.name, "main.rs")]);
  }
}
let failed = 0;
for (const [source, expected] of programs) {
  const run = spawnSync(process.execPath, [bin, source, "--no-header"], { encoding: "utf8" });
  if (run.status !== 0 || run.stdout !== fs.readFileSync(expected, "utf8")) {
    failed++;
    console.error(`differs: ${path.relative(cases, source)}\n${run.stderr}`);
  }
}
console.log(`${programs.length - failed} of ${programs.length} programs translated as by the native build`);
process.exit(failed === 0 ? 0 : 1);
