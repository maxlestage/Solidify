#!/usr/bin/env node
// `npx solidify-rs main.ts -o app.rs`: Solidify compiled to WebAssembly (WASI), run by
// Node.js with the files of this machine. Same options as the native command.
"use strict";

const fs = require("node:fs");
const path = require("node:path");

// `node:wasi` announces that it is experimental: nothing for the user to act on.
const emitWarning = process.emitWarning;
process.emitWarning = function (warning, ...rest) {
  const type = typeof rest[0] === "string" ? rest[0] : rest[0] && rest[0].type;
  if (type === "ExperimentalWarning" && String(warning).includes("WASI")) return;
  return emitWarning.call(process, warning, ...rest);
};
const { WASI } = require("node:wasi");

// The program sees the file system from its root: paths are made absolute first.
const root = path.parse(process.cwd()).root;
const inside = (file) => {
  const absolute = path.resolve(file);
  return "/" + path.relative(root, absolute).split(path.sep).join("/");
};

const args = [];
const given = process.argv.slice(2);
for (let i = 0; i < given.length; i++) {
  const arg = given[i];
  if ((arg === "-o" || arg === "--output") && i + 1 < given.length) {
    args.push(arg, inside(given[++i]));
  } else if (arg === "-" || arg.startsWith("-")) {
    args.push(arg);
  } else {
    args.push(inside(arg));
  }
}

const wasi = new WASI({
  version: "preview1",
  args: ["solidify", ...args],
  env: process.env,
  preopens: { "/": root },
  returnOnExit: true,
});
const wasm = fs.readFileSync(path.join(__dirname, "..", "solidify.wasm"));
const instance = new WebAssembly.Instance(new WebAssembly.Module(wasm), wasi.getImportObject());
process.exitCode = wasi.start(instance);
