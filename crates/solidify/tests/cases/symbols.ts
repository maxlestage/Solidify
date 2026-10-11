// Symbols: iterable classes ([Symbol.iterator], generator methods), Symbol() values,
// Symbol.for, updates used as values.

class Range {
  constructor(private from: number, private to: number) {}
  *[Symbol.iterator]() {
    for (let i = this.from; i <= this.to; i++) yield i;
  }
}
class Countdown {
  constructor(private n: number) {}
  [Symbol.iterator]() {
    let k = this.n;
    return {
      next: () => (k > 0 ? { value: k--, done: false } : { value: 0, done: true }),
    };
  }
}
const r = new Range(1, 4);
for (const x of r) console.log(x);
console.log([...new Range(2, 3)], Array.from(new Countdown(3)));
const id = Symbol("id");
const other = Symbol("id");
console.log(id.toString(), id.description, id === other, id === id, typeof id);
const tags = new Map<symbol, string>();
tags.set(id, "first");
console.log(tags.get(id), tags.get(other), Symbol.for("app") === Symbol.for("app"));
console.log(Symbol.keyFor(Symbol.for("app")), Symbol.keyFor(id), String(Symbol()));
const xs = [10, 20, 30];
let i = 0;
console.log(xs[i++], xs[i++], i, xs[--i]);
let a: number;
let b: number;
a = b = 5;
console.log(a + b);
