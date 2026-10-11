const items = [{ name: "a", qty: 2 }, { name: "b", qty: 3 }];
for (const { name, qty } of items) console.log(name, qty);
const nested = { user: { id: 1, tags: ["x", "y"] } };
const { user: { id, tags: [first, second] } } = nested;
console.log(id, first, second);
let p = 0, q = 0;
({ p, q } = { p: 1, q: 2 });
console.log(p, q);
const pairs: [string, { n: number }][] = [["a", { n: 1 }]];
for (const [key, { n }] of pairs) console.log(key, n);
function area({ size: { w, h } }: { size: { w: number; h: number } }): number {
  return w * h;
}
console.log(area({ size: { w: 2, h: 3 } }));
const { name, ...rest } = items[0];
console.log(name, rest);
const cfg: Record<string, number> = { a: 1, b: 2, c: 3 };
const { a, ...others } = cfg;
console.log(a, others);
function f({ x, ...more }: { x: number; y: string; z: boolean }) {
  console.log(x, more);
}
f({ x: 1, y: "s", z: true });
const people = [{ id: 1, nm: "x", age: 3 }];
for (const { id, ...info } of people) console.log(id, info);
