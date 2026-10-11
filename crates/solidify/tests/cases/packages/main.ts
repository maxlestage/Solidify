// CommonJS files (`require`, `module.exports`), a package from node_modules, and Node's
// built-in modules.
const { greet, loud } = require("./lib/greet");
const Counter = require("./lib/counter");
const path = require("path");
const fsp = require("fs/promises");
const { clamp, PHI } = require("tiny-math");
const { argv, platform } = require("node:process");

console.log(greet("ada"), loud("hi"));
const c = new Counter();
c.inc();
console.log(c.inc(), path.join("x", "y"), clamp(12, 0, 10), PHI);
console.log(argv.length > 0, platform === process.platform);

async function main() {
  const file = path.join(require("os").tmpdir(), "solidify-packages.txt");
  await fsp.writeFile(file, "data");
  console.log(await fsp.readFile(file, "utf8"));
  await fsp.rm(file);
}
main();
