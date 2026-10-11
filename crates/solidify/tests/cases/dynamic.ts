// Values known only at run time: JSON without a type, revivers and replacers.

interface Item {
  name: string;
  price: number;
  tags: string[];
}

const raw =
  '{"items":[{"name":"pen","price":1.5,"tags":["a"]},{"name":"book","price":12,"tags":[]}],"owner":null,"count":2}';
const data = JSON.parse(raw);

console.log(data.items.map((i) => i.name.toUpperCase()));
const cheap = data.items.filter((i) => i.price < 5);
console.log(cheap.length, cheap[0].name);
console.log(data.items.reduce((sum, i) => sum + i.price, 0));
console.log(`${data.items[1].name} costs ${data.items[1].price}`);

// Read with a type: checked and converted.
const first: Item = data.items[0];
console.log(first.name, first.price * 2, first.tags.length);
const all: Item[] = data.items;
for (const it of all) console.log(`${it.name}: ${it.price.toFixed(2)}`);

console.log(data.owner === null, data.owner ?? "nobody", data.missing?.deep);
console.log(typeof data.count, typeof data.items, typeof data.missing);

// Changed in place.
data.items.push({ name: "cup", price: 3, tags: [] });
data.count += 1;
data.count++;
data.items[0].price = 2;
data.items.forEach((i, idx) => console.log(idx, i.name, i.price));
const keys: string[] = [];
for (const k in data) keys.push(k);
console.log(keys.join(","), Object.keys(data.items[0]));
console.log(JSON.stringify(data));

const cfg = JSON.parse('{"user":"ada","retries":"3","debug":false,"list":[3,1,2]}');
function greet(name: string, times: number): string {
  return `hi ${name} x${times}`;
}
console.log(greet(cfg.user, cfg.list.length));
console.log(cfg.retries * 2, cfg.retries + 1, +cfg.retries, Number(cfg.retries));
console.log(cfg.list.join("-"), cfg.list.includes(2), cfg.list.sort((a, b) => a - b));
if (!cfg.debug) console.log("not debug", Array.isArray(cfg.list));
let total = 0;
for (const v of cfg.list) total += v;
console.log(total);

// A reviver sees every value, innermost first.
const revived = JSON.parse('{"name":"Ada","age":36,"born":{"year":1815}}', (key, value) =>
  typeof value === "number" ? value * 2 : value,
);
console.log(revived);
interface Point {
  x: number;
  y: number;
}
const p: Point = JSON.parse('{"x":1,"y":2}', (k, v) => (k === "x" ? v + 10 : v));
console.log(p.x + p.y);

// A replacer: a function, or the keys to keep.
const user = { name: "Ada", password: "secret", age: 36 };
console.log(JSON.stringify(user, (key, value) => (key === "password" ? undefined : value)));
console.log(JSON.stringify(user, ["name", "age"], 2));
console.log(JSON.stringify({ a: [1, 2], b: "x" }, (k, v) => (typeof v === "number" ? v * 10 : v)));

// Built at run time.
const o = JSON.parse("{}");
o.name = "x";
o.list = [1, "two", null];
o.nested = { a: 1, b: [true, null] };
o["key" + 1] = "computed";
console.log(o);
