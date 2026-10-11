// Unions of different types become enums; tests of a member narrow the value.
type Shape =
  | { kind: "circle"; radius: number }
  | { kind: "square"; side: number }
  | { kind: "rect"; width: number; height: number };

function area(shape: Shape): number {
  switch (shape.kind) {
    case "circle":
      return Math.PI * shape.radius * shape.radius;
    case "square":
      return shape.side * shape.side;
    case "rect":
      return shape.width * shape.height;
  }
}

function describe(value: string | number): string {
  if (typeof value === "string") {
    return `text of ${value.length} chars`;
  }
  return `number ${value * 2}`;
}

function first(items: string | string[]): string {
  if (Array.isArray(items)) {
    return items.length > 0 ? items[0] : "";
  }
  return items;
}

interface Cat {
  name: string;
  lives: number;
}

interface Robot {
  name: string;
  battery: number;
}

type Pet = Cat | Robot;

function charge(pet: Pet): string {
  if ("battery" in pet) {
    pet.battery += 10;
    return `${pet.name} charged to ${pet.battery}`;
  }
  return `${pet.name} has ${pet.lives} lives`;
}

function label(id: number | string | boolean): string {
  if (typeof id === "boolean") {
    return id ? "yes" : "no";
  }
  if (typeof id !== "number") {
    return id.toUpperCase();
  }
  return (id + 1).toString();
}

const shapes: Shape[] = [
  { kind: "circle", radius: 1 },
  { kind: "square", side: 2 },
  { kind: "rect", width: 2, height: 3 },
];
for (const s of shapes) {
  console.log(s.kind, area(s).toFixed(2));
}
console.log(describe("hello"), describe(21), first(["a", "b"]), first("solo"), first([]));
const pets: Pet[] = [{ name: "Tom", lives: 9 }, { name: "R2", battery: 50 }];
for (const pet of pets) {
  console.log(charge(pet));
}
console.log(label(41), label("ok"), label(true), label(false));
let mixed: (number | string)[] = [1, "two", 3];
mixed.push("four");
mixed = mixed.filter((m) => typeof m === "number" || m.length > 3);
console.log(mixed, typeof mixed[0], typeof "x", typeof 1.5);
const picked: Shape = shapes.length > 2 ? { kind: "square", side: 5 } : { kind: "circle", radius: 2 };
console.log(picked, picked.kind === "square", area(picked));
let value: string | number = 5;
value = "five";
console.log(value, value === "five", value === 5);

// Tagged tokens, else-if chains, nullable unions.
type Token = { type: "num"; value: number } | { type: "op"; symbol: string } | { type: "end" };

class Lexer {
  tokens: Token[] = [];
  current: Token | null = null;

  push(t: Token): void {
    this.tokens.push(t);
    this.current = t;
  }

  describe(): string {
    const parts: string[] = [];
    for (const t of this.tokens) {
      if (t.type === "num") {
        parts.push(`num(${t.value})`);
      } else if (t.type === "op") {
        parts.push(`op(${t.symbol})`);
      } else {
        parts.push("end");
      }
    }
    return parts.join(" ");
  }
}

function numberOf(t: Token): number {
  if (t.type !== "num") {
    return 0;
  }
  return t.value;
}

function show(x: number | string | boolean | null): string {
  if (x === null) return "null";
  if (typeof x === "number") return `n:${x}`;
  if (typeof x === "string") return `s:${x}`;
  return `b:${x}`;
}

const lx = new Lexer();
lx.push({ type: "num", value: 4 });
lx.push({ type: "op", symbol: "+" });
lx.push({ type: "end" });
console.log(lx.describe(), lx.tokens.length, numberOf(lx.tokens[0]), numberOf(lx.tokens[1]));
const found = lx.tokens.find((t) => t.type === "op");
console.log(found, show(3), show("x"), show(false), show(null));
const kinds = lx.tokens.map((t) => t.type);
console.log(kinds, lx.current);

// An array literal of different types is an array of their union.
const items = [1, "two", 3.5, "four"];
for (const m of items) {
  if (typeof m === "string") {
    console.log(m.toUpperCase());
  } else {
    console.log(m * 2);
  }
}
console.log(items, items.length);
const nested = [1, [2, 3], "x"];
console.log(nested, JSON.stringify(nested));
