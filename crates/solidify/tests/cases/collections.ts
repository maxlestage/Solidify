// Maps, sets, records and tuples.
const stock = new Map<string, number>();
stock.set("apple", 3);
stock.set("pear", 0);
stock.set("fig", 12);
if (stock.has("apple")) {
  console.log("apples:", stock.get("apple"));
}
const missing = stock.get("kiwi");
console.log(missing === undefined ? "no kiwi" : "kiwi!");
let inStock = 0;
for (const [name, count] of stock) {
  if (count > 0) inStock += 1;
  if (name === "fig") console.log("figs:", count);
}
console.log("in stock:", inStock, "kinds:", stock.size);

const seen = new Set<number>();
for (const n of [1, 2, 2, 3, 3, 3]) {
  seen.add(n);
}
console.log(seen.size, seen.has(2), seen.has(9));

const ages: Record<string, number> = { ann: 31, bob: 27 };
ages["cid"] = 45;
const names = Object.keys(ages);
names.sort();
for (const n of names) {
  console.log(n, ages[n]);
}

const pair: [string, number] = ["x", 10];
const [label, value] = pair;
console.log(label, value * 2, pair[0]);
