function makeCounter(): () => number {
  let count = 0;
  return () => {
    count += 1;
    return count;
  };
}

function apply(f: (x: number) => number, value: number): number {
  return f(value);
}

const double = (x: number) => x * 2;
const next = makeCounter();
next();
next();
console.log(next(), apply(double, 21), apply((x) => x + 1, 1));

const greet = (name: string, excited = false): string => {
  const base = `Hello, ${name}`;
  return excited ? base + "!" : base;
};
console.log(greet("Ada"), greet("Bob", true));

function first<T>(items: T[]): T {
  return items[0];
}
function same<T>(a: T, b: T): boolean {
  return a === b;
}
console.log(first([7, 8]), first(["a", "b"]), same(1, 1), same("x", "y"));

let hits = 0;
[1, 2, 3].forEach((n) => {
  if (n % 2 === 1) hits++;
});
console.log(hits);
