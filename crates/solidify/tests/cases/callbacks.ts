interface Op {
  name: string;
  apply: (x: number) => number;
}
const ops: Op[] = [
  { name: "inc", apply: (x) => x + 1 },
  { name: "double", apply: (x) => x * 2 },
];
for (const op of ops) {
  console.log(op.name, op.apply(5));
}
const firstOp = ops[0];
console.log(firstOp.apply(10), ops.map((o) => o.apply(1)));
class Button {
  constructor(public label: string, private onClick: () => void) {}
  click(): void {
    this.onClick();
  }
}
const b = new Button("ok", () => console.log("clicked"));
b.click();
console.log(b.label, ops[1]);

// Optional calls: nothing when the function or the object is missing.
interface User {
  name: string;
  address?: { city: string; zip?: string };
  greet?: () => string;
  score?: (n: number) => number;
}
class Pet {
  constructor(public name: string) {}
  speak(): string {
    return `${this.name} speaks`;
  }
  rename(n: string): void {
    this.name = n;
  }
}
const u: User = { name: "a", address: { city: "Paris" }, greet: () => "hi" };
const v: User = { name: "b" };
console.log(u.address?.city, u.address?.zip ?? "-", u.greet?.() ?? "no greet", v.greet?.() ?? "none");
console.log(u.score?.(2) ?? -1, [u, v].map((x) => x.address?.city ?? "?").join(","));
let pet: Pet | null = new Pet("Rex");
console.log(pet?.speak() ?? "silent");
pet?.rename("Max");
console.log(pet?.name, pet?.speak());
pet = null;
console.log(pet?.speak() ?? "silent", pet?.name);
