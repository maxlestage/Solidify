function collatz(n: number): number {
  let steps = 0;
  while (n !== 1) {
    n = n % 2 === 0 ? n / 2 : 3 * n + 1;
    steps++;
  }
  return steps;
}

function primes(limit: number): number[] {
  const out: number[] = [];
  outer: for (let i = 2; i <= limit; i++) {
    for (let j = 2; j * j <= i; j++) {
      if (i % j === 0) continue outer;
    }
    out.push(i);
  }
  return out;
}

let i = 0;
do {
  i += 3;
} while (i < 10);
console.log(i, collatz(27), primes(30).join(" "));

for (let k = 10; k > 0; k -= 3) {
  if (k === 4) continue;
  console.log(k);
}
const grid = [[1, 2], [3, 4]];
let sum = 0;
for (let r = 0; r < grid.length; r++) {
  for (let c = 0; c < grid[r].length; c++) {
    sum += grid[r][c];
  }
}
console.log(sum, grid[1][0], grid.length);
let count = 0;
for (;;) {
  count++;
  if (count > 4) break;
}
console.log(count, 7 % 3, -7 % 3, 2 ** 10, Math.floor(7 / 2), 7 / 2);
