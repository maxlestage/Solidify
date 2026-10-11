const values = [3.5, -2.25, 10, 0.5];
let acc = 0;
for (const v of values) {
  acc += Math.abs(v);
}
console.log(acc, Math.floor(acc), Math.ceil(2.1), Math.round(2.5), Math.trunc(-2.7));
console.log(Math.max(1, 7, 3), Math.min(4, 2), Math.sqrt(16), Math.pow(2, 8), 2 ** 0.5 > 1.41);
console.log(Math.max(...values), Math.min(...values), Math.sign(-3), Math.hypot(3, 4));
const n = 7;
console.log(n / 2, Math.floor(n / 2), n % 4, (n * 3) / 3, n & 3, n | 8, n ^ 1, n << 2, n >> 1, ~n);
console.log(Math.PI.toFixed(3), Number.isInteger(4), Number.isInteger(4.5), isNaN(parseFloat("x")), (255).toString(16));
console.log(Number.MAX_SAFE_INTEGER, 1e3, 0.1 + 0.2 === 0.3, 5 > 3 && 2 < 1, !(1 > 2));
