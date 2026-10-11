// Numbers as JavaScript has them: no overflow, 32-bit bitwise operators, exact formatting.

function fact(n: number): number {
  return n <= 1 ? 1 : n * fact(n - 1);
}
function fib(n: number): number {
  let a = 0;
  let b = 1;
  for (let i = 0; i < n; i++) {
    [a, b] = [b, a + b];
  }
  return a;
}
let p = 1;
for (let i = 0; i < 70; i++) {
  p *= 3;
}
let h = 0;
for (const c of "hello world") {
  h = (h * 31 + c.charCodeAt(0)) | 0;
}
let sum = 0;
for (const x of [1, 2, 3]) {
  sum += x;
}
console.log(fact(25), fib(100), p, h, sum);
console.log(1 << 31, 5 & 3, -7 >> 1, -7 >>> 0, ~5, 2 ** 64, 0xffffffff | 0);
console.log(Number.MAX_SAFE_INTEGER + 2, 9007199254740993);

const xs = [1.5, 2, 1e21, 1e-7, -0];
console.log(xs.join("-"), `${1 / 0}`, String(0.1 + 0.2), 1e21, 1 / 3, -1 / 0, 0 / 0);
const big = 123456789 * 1000000000000;
console.log(big, (2 ** 53) + 1, 5 % 3, -5 % 3, 7.5 % 2);
let acc = 1;
for (let i = 1; i <= 30; i++) acc = acc * i;
console.log(acc, acc.toString(), acc > 1e30);
const vals = [2.5, -2.5, 1.005, 1.25, 0.000001234, 123.456, 1e21, 0, -0.0001, 99.995, 1234.5678];
for (const v of vals) {
  console.log(v.toFixed(0), v.toFixed(2), v.toPrecision(3), v.toPrecision(1), v.toString());
}
console.log((25).toFixed(1), (7).toPrecision(2), (123456).toPrecision(2), (0.00000123).toPrecision(2));
