// parseInt gives an optional integer: NaN is None, and isNaN tests it.
function toInt(s: string): number {
  const n = parseInt(s);
  if (isNaN(n)) {
    return -1;
  }
  return n * 2;
}

function firstDigits(s: string): string {
  const n = parseInt(s, 10);
  if (!isNaN(n)) {
    return `number ${n}`;
  }
  return "no number";
}

function hex(s: string): number {
  return parseInt(s, 16) || 0;
}

// `any` and `unknown` are inferred from their uses.
function twice(x: any) {
  return x + x;
}

function shout(s: any) {
  return s.toUpperCase() + "!";
}

function label(value: unknown, times: number): string {
  let out = "";
  for (let i = 0; i < times; i++) {
    out += `${value}`;
  }
  return out;
}

const raw: any = "12";
const parsed = parseInt(raw) || 0;
console.log(toInt("21"), toInt("x"), toInt("  7 apples"));
console.log(firstDigits("abc"), firstDigits("42abc"), hex("ff"), hex("0x1A"), hex("zz"));
console.log(twice(4), shout("ab"), label("na", 3), label(1.5, 2), parsed + 1);
console.log(parseInt("3.9"), parseInt(3.9), parseInt(-2.5), Number.parseInt("8"), Number.isNaN(parseInt("q")));
const maybe = Number.parseInt("1e3");
console.log(isFinite(maybe) ? "finite" : "nan", isNaN(parseInt("nope")));
