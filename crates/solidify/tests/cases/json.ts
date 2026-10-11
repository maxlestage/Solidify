interface Address {
  city: string;
  zip?: string;
}

interface User {
  name: string;
  age: number;
  tags: string[];
  address: Address;
  nickname: string | null;
  scores: Record<string, number>;
}

enum Color {
  Red = "red",
  Green = "green",
}

type Shape =
  | { kind: "circle"; radius: number }
  | { kind: "square"; side: number };

class Point {
  label?: string;
  constructor(
    public x: number,
    public y: number,
  ) {}
  norm(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y);
  }
}

function area(s: Shape): number {
  switch (s.kind) {
    case "circle":
      return Math.PI * s.radius * s.radius;
    case "square":
      return s.side * s.side;
  }
}

function parseUser(text: string): User | null {
  try {
    return JSON.parse(text);
  } catch (e) {
    console.log("bad user:", e.message);
    return null;
  }
}

const text = `{"name": "Ada", "age": 36, "tags": ["math", "code"],
  "address": {"city": "London"}, "nickname": null,
  "scores": {"logic": 10, "poetry": 7.5}}`;
const user: User = JSON.parse(text);
console.log(user.name, user.age, user.tags.length, user.address.city);
console.log(user.address.zip ?? "no zip", user.nickname ?? "no nickname");
for (const [subject, score] of Object.entries(user.scores)) {
  console.log(subject, score);
}
console.log(JSON.stringify(user));
console.log(JSON.stringify(user, null, 2));
console.log(JSON.stringify(user.scores));

const shapes: Shape[] = JSON.parse(
  `[{"kind": "circle", "radius": 1}, {"kind": "square", "side": 2}]`,
);
for (const s of shapes) {
  console.log(s.kind, area(s).toFixed(2));
}
console.log(JSON.stringify(shapes));

const colors: Color[] = JSON.parse(`["red", "green", "red"]`);
console.log(colors, JSON.stringify(colors));

const mixed: (number | string)[] = JSON.parse(`[1, "two", 3.5]`);
for (const m of mixed) {
  if (typeof m === "number") {
    console.log("number", m * 2);
  } else {
    console.log("string", m.toUpperCase());
  }
}

const pair: [string, number] = JSON.parse(`["x", 42]`);
console.log(pair[0], pair[1]);

const p = new Point(3, 4);
console.log(JSON.stringify(p));
p.label = "hyp";
console.log(JSON.stringify(p), p.norm());
const q: Point = JSON.parse(JSON.stringify(p));
console.log(q.x + q.y, q.label);

const data = {
  a: [1, 2, 3],
  b: { c: 'c"d\n', d: [] as string[] },
  f: true,
  g: Math.sqrt(-1),
  h: 1.5e-7,
  i: new Map([["k", 1]]),
};
console.log(JSON.stringify(data));
console.log(JSON.stringify(data, null, "\t"));
console.log(
  JSON.stringify('he said "hi"'),
  JSON.stringify(3.5),
  JSON.stringify(null),
  JSON.stringify(true),
);
console.log(JSON.stringify([[1, 2], [3]], null, 4));

console.log(parseUser("{") === null);
for (const bad of ["", "[1,]", '{"a":1,}', "nope", '{"a" 1}', '"open', "01", "[1,2]x"]) {
  try {
    const v: number[] = JSON.parse(bad);
    console.log("parsed", v);
  } catch (e) {
    console.log(e.message);
  }
}
