function html(strings: TemplateStringsArray, ...values: (string | number)[]): string {
  let out = "";
  strings.forEach((s, i) => {
    out += s;
    if (i < values.length) out += `[${values[i]}]`;
  });
  return out;
}
const who = "Ann";
console.log(html`Hello ${who}, you are ${42}!`);
const Point = class {
  constructor(public x: number, public y: number) {}
  norm(): number { return Math.abs(this.x) + Math.abs(this.y); }
};
console.log(new Point(3, -4).norm());
class Config {
  static values: string[] = [];
  static {
    this.values.push("a");
    Config.values.push("b");
  }
}
console.log(Config.values);
type Key = `id-${number}`;
const k: Key = "id-7";
console.log(k);
async function main() {
  const jobs = [Promise.resolve(1), Promise.resolve(2)];
  for await (const v of jobs) console.log("got", v);
  for await (const n of [3, 4]) console.log("plain", n);
}
main();
class Counter {
  static count = 0;
  static names: string[] = [];
  static readonly LIMIT = 10;
  id: number;
  constructor(name: string) {
    Counter.count++;
    this.id = Counter.count;
    Counter.names.push(name);
  }
  static reset(): void {
    this.count = 0;
  }
  static total(): number {
    return Counter.count * Counter.LIMIT;
  }
}
class Special extends Counter {}
new Counter("a");
new Special("b");
console.log(Counter.count, Counter.names, Special.count, Counter.total());
Counter.reset();
console.log(Counter.count, Counter.LIMIT);
const xs = ["a", "b"];
let out = "";
xs.forEach((s, i) => {
  out += s;
  if (i < 1) out += "-";
});
console.log(out);
const prices = "USD 10, EUR 20, USD 35";
console.log(prices.match(/(?<=USD )\d+/g));
console.log("a1b2c3".replace(/(?<![ab])\d/g, "#"));
console.log(/(?<=\$)\d+(\.\d+)?/.exec("cost: $42.50")?.[0]);
