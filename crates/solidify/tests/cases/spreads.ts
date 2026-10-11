interface Point {
  x: number;
  y: number;
}

// Rest parameters take the extra arguments; a spread passes an array.
function sum(...nums: number[]): number {
  return nums.reduce((a, b) => a + b, 0);
}

function tag(label: string, ...parts: string[]): string {
  return label + ": " + parts.join("/");
}

const xs = [1, 2, 3];
console.log(sum(...xs), sum(1, 2), sum(), Math.max(...xs), Math.min(...xs));
console.log(tag("a"), tag("b", "x", "y"));

// Object spread: the same shape, or a new one.
const base: Point = { x: 1, y: 2 };
const ext = { ...base, z: 3 };
const moved = { ...base, x: 9 };
console.log(ext, moved, ext.z + moved.x);

function shift(p: Point): Point {
  return { ...p, x: p.x + 1 };
}
console.log(shift(base), { ...ext, w: "w" }, { ...moved, ...ext });

// Destructuring with a rest element, and in parameters.
const [first, ...rest] = xs;
console.log(first, rest, rest.length);

function dist({ x, y }: Point): number {
  return Math.sqrt(x * x + y * y);
}
const swap = ([a, b]: [string, number]): string => `${b}${a}`;
console.log(dist({ x: 3, y: 4 }), dist(base), swap(["s", 7]));

// Methods that change an array give it back.
console.log([3, 1, 2].sort((a, b) => a - b), ["b", "a"].sort(), [1, 2, 3].reverse(), new Array(3).fill(0), Array(2).fill("x"));
const nums = [5, 4, 6];
const sorted = [...nums].sort((a, b) => a - b);
console.log(sorted, nums.reverse(), nums);

// A property's getter and setter share one type.
class Temp {
  private c = 0;
  get f(): number {
    return (this.c * 9) / 5 + 32;
  }
  set f(v: number) {
    this.c = ((v - 32) * 5) / 9;
  }
  toString(): string {
    return `${this.c.toFixed(1)}C`;
  }
}
const t = new Temp();
t.f = 212;
console.log(t.f, `${t}`, String(t));
