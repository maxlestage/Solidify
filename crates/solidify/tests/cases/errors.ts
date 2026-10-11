function parsePositive(s: string): number {
  const n = parseFloat(s);
  if (isNaN(n)) {
    throw new Error(`not a number: ${s}`);
  }
  if (n < 0) throw new Error("negative");
  return n;
}

function sum(items: string[]): number {
  let t = 0;
  for (const item of items) {
    t += parsePositive(item);
  }
  return t;
}

try {
  console.log(sum(["1", "2.5", "3"]));
  console.log(sum(["1", "x"]));
} catch (e) {
  console.log("failed:", e.message);
}
