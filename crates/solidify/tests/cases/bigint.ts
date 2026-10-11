// BigInt: integers of any size.

function factorial(n: bigint): bigint {
  let r = 1n;
  for (let i = 2n; i <= n; i++) r *= i;
  return r;
}
console.log(factorial(30n));
console.log(`25! = ${factorial(25n)}`);
const big = 2n ** 128n - 1n;
console.log(big, typeof big, big.toString(16), big % 1000n);
const a: bigint = BigInt("123456789012345678901234567890");
const b = BigInt(42);
console.log(a / b, a % b, -b, b * b > 1000n, b < 100, b == 42n, a === a);
console.log([1n, 2n, 3n], { id: 7n }, Number(b) + 0.5, String(b), BigInt(Number.MAX_SAFE_INTEGER) + 2n);
let x = 10n;
x += 5n;
x--;
console.log(x, x & 0xffn, x << 3n, -x >> 2n, x ^ 7n);
const ids = new Map<bigint, string>();
ids.set(1n, "one");
console.log(ids.get(1n), ids.has(2n));
