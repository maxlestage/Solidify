// Generators are lazy: each `next()` runs the body up to the next `yield`.
function* range(start: number, end: number, step = 1): Generator<number> {
  for (let i = start; i < end; i += step) {
    yield i;
  }
}

function* naturals(): Generator<number> {
  let n = 1;
  while (true) {
    yield n;
    n++;
  }
}

function* fibonacci(): Generator<number> {
  let [a, b] = [0, 1];
  while (true) {
    yield a;
    [a, b] = [b, a + b];
  }
}

function* words(text: string): Generator<string> {
  for (const w of text.split(" ")) {
    if (w.length > 0) {
      yield w.toUpperCase();
    }
  }
}

function* chained(): Generator<number> {
  yield* range(0, 3);
  yield* [10, 20];
  yield 99;
}

function take(count: number, gen: Generator<number>): number[] {
  const out: number[] = [];
  for (const value of gen) {
    if (out.length >= count) {
      break;
    }
    out.push(value);
  }
  return out;
}

for (const i of range(0, 10, 3)) {
  console.log("range", i);
}
console.log([...range(1, 4)], Array.from(words("hello big  world")), [...chained()]);
console.log(take(5, naturals()), take(8, fibonacci()));
const it = range(5, 7);
const first = it.next();
console.log(first.value, first.done, it.next().value, it.next().done);
let sum = 0;
for (const f of fibonacci()) {
  if (f > 100) break;
  sum += f;
}
console.log(sum);
