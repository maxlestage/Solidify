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
