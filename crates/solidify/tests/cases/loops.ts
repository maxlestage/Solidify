function search(grid: number[][], target: number): [number, number] | null {
  outer: for (let r = 0; r < grid.length; r++) {
    for (let c = 0; c < grid[r].length; c++) {
      if (grid[r][c] === target) {
        return [r, c];
      }
      if (grid[r][c] > target) continue outer;
    }
  }
  return null;
}

function describe(n: number): string {
  let kind = "";
  switch (n % 3) {
    case 0:
      kind = "fizz";
      break;
    case 1: {
      kind = "one";
      break;
    }
    default:
      kind = "other";
  }
  return kind;
}

function firstNegative(xs: number[]): number {
  let i = 0;
  while (true) {
    if (i >= xs.length) {
      return -1;
    }
    if (xs[i] < 0) break;
    i += 1;
  }
  return i;
}

const grid = [
  [1, 3, 5],
  [2, 4, 6],
];
const hit = search(grid, 4);
if (hit !== null) {
  const [r, c] = hit;
  console.log(`found at ${r},${c}`);
}
console.log(search(grid, 7) === null, describe(9), describe(10), describe(11));
console.log(firstNegative([3, 1, -2, 5]), firstNegative([1, 2]));
let evens = 0;
for (let i = 10; i >= 0; i -= 2) {
  evens++;
  if (i === 4) break;
}
const names = ["ann", "bob"];
for (const [i, n] of names.entries()) {
  console.log(i, n);
}
const counts = new Map<string, number>([["a", 1]]);
for (const key of counts.keys()) {
  counts.set(key, counts.get(key)! + 1);
}
console.log(evens, counts.get("a"), [3, 1, 2].toSorted((a, b) => a - b).join(""));
