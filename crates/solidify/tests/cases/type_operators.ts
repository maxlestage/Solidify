type A = { a: number };
type B = { b: string };
type AB = A & B;
const v: AB = { a: 1, b: "x" };
interface Dict { [key: string]: number }
const d: Dict = { x: 1 };
d["y"] = 2;
type K = keyof A;
const k: K = "a";
console.log(v.a, v.b, d, k);
interface Named { name: string }
interface Aged { age: number }
type Person = Named & Aged & { email?: string };
function greet(p: Named & Aged): string {
  return `${p.name} (${p.age})`;
}
const bob: Person = { name: "Bob", age: 42 };
console.log(greet({ name: "Ann", age: 30 }), bob);
type UserId = string & { readonly __brand: "UserId" };
const id = "u-1" as UserId;
console.log(id.length, id);
interface Scores { [player: string]: number }
const scores: Scores = {};
scores["ann"] = 3;
scores.bob = 5;
for (const k in scores) console.log(k, scores[k]);
type Age = Person["age"];
const a: Age = 7;
type Field = keyof Person;
const fields: Field[] = ["name", "age"];
console.log(a + 1, fields);
const base = { x: 1, y: 2 };
const copy: typeof base = { x: 3, y: 4 };
console.log(copy);
type Pair = [string, number];
const second: Pair[1] = 9;
console.log(second);
