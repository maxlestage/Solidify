// Missing values: `undefined` and `null` printed as Node does, from where they come.
interface Item { name: string; note?: string; next: Item | null; extra: number | undefined }
function f(xs: number[]): number | undefined {
  return xs.length > 0 ? xs[0] : undefined;
}
function g(b: boolean): string | null {
  return b ? "yes" : null;
}
function h(x?: number) {
  console.log(x);
}
const a: Item = { name: "a", next: null, extra: undefined };
const b: Item = { name: "b", note: "n", next: a, extra: 2 };
let u: number | undefined;
let n: number | null = null;
const m = new Map<string, number>();
console.log(f([]), f([3]), g(true), g(false));
h();
h(2);
console.log(u, n, m.get("x"), a.note, b.note, a.extra);
console.log(a, b);
console.log([f([]), f([1])], [g(false)]);
const found = [1, 2].find((x) => x > 5);
console.log(found, `${found}`, `${g(false)}`);
const looked = ["a", "z"].map((k) => m.get(k));
console.log(looked, `${looked}`);
