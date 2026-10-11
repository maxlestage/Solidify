// `any` and `unknown` values, records read with a dot, objects written with methods.

function describe(value: unknown): string {
  if (typeof value === "number") return `number ${value * 2}`;
  if (typeof value === "string") return `string ${value.toUpperCase()}`;
  if (Array.isArray(value)) return `array of ${value.length}`;
  if (value === null) return "null";
  return "other " + JSON.stringify(value);
}
console.log(describe(21), describe("hi"), describe([1, 2]), describe(null), describe({ a: 1 }));
const bag: any[] = [1, "two", true, { three: 3 }];
for (const item of bag) console.log(typeof item, item);
let anything: any = 5;
anything = anything + 1;
anything = "now text";
console.log(anything, anything.length);
const record: Record<string, any> = { n: 1, s: "x" };
console.log(record.n + 1, record["s"]);

const prefix = ">";
const logger = {
  lines: [] as string[],
  log(msg: string) {
    this.lines.push(msg);
  },
  get count() {
    return this.lines.length;
  },
};
logger.log("a");
logger.log("b");
console.log(logger.count, logger.lines);
console.log(logger);
function makePoint(x: number, y: number) {
  return {
    x,
    y,
    norm() {
      return Math.sqrt(this.x * this.x + this.y * this.y);
    },
  };
}
const p = makePoint(3, 4);
console.log(p.norm(), p.x);

const o = { a: 1, f() { return this.a; }, get g() { return 2; }, b: "x", set g(v: number) {} };
console.log(o, o.f(), o.g);
