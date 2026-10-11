// Classic recursion and a counted loop.
function fib(n: number): number {
  if (n < 2) return n;
  return fib(n - 1) + fib(n - 2);
}

let total = 0;
for (let i = 0; i < 10; i++) {
  total += fib(i);
}
console.log("sum of the first ten:", total);
console.log(`fib(20) = ${fib(20)}`);
