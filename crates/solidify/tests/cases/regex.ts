// Regular expressions: a small engine written into the program, with JavaScript's syntax.
const email = /^([\w.+-]+)@([\w-]+)\.([a-z]{2,})$/i;
const dateRe = /(\d{4})-(\d{2})-(\d{2})/;

function check(address: string): string {
  const m = address.match(email);
  if (m === null) {
    return `${address}: invalid`;
  }
  return `${address}: user=${m[1]} host=${m[2]} tld=${m[3].toLowerCase()}`;
}

console.log(check("Ada.Lovelace+math@Example.COM"), check("nope@"), email.test("x@y.io"));
const text = "Dates: 2024-01-15, 1999-12-31 and 2000-02-29.";
const found = text.match(/\d{4}-\d{2}-\d{2}/g);
console.log(found, found === null ? 0 : found.length, text.search(dateRe));
for (const m of text.matchAll(/(\d{4})-(\d{2})/g)) {
  console.log(m[0], "year", m[1], "month", m[2], "at", m.index);
}
const first = dateRe.exec(text);
if (first !== null) {
  console.log(first[0], first.index, first.length, first[1] + "/" + first[2]);
}
console.log(text.replace(dateRe, "$3/$2/$1"), text.replace(/\d+/g, "#"));
console.log("hello world".replace(/\b\w/g, (c) => c.toUpperCase()), "a-b_c".replace(/[-_]/g, " "));
console.log("2024-06-01".replace(/(\d+)-(\d+)-(\d+)/, (all, y, mo, d) => `${d}.${mo}.${y} (${all.length})`));
console.log("a1b22c333".split(/\d+/), "one, two,three ,four".split(/\s*,\s*/).join("|"), "x".split(/,/));
const words = "The quick brown fox".split(/\s+/);
console.log(words.length, words.map((w) => w.length).join(","), /quick/.source, /x/gi.flags, /x/g.global);
const re = new RegExp("c.t", "g");
console.log("cat cot cut".replace(re, "-"), re.test("dog"), new RegExp("^a+$").test("aaa"));
console.log("tabs\tand\nnewlines".split(/\s/).length, "mississippi".replace(/s+/g, "S"), /(?<word>\w+)/.exec("hi there")?.[1]);
console.log("price: 42 EUR".replace("42", (n) => (parseInt(n)! * 2).toString()), "aaa".replaceAll("a", (x) => x.toUpperCase()));
console.log(/^\p{L}+$/u.test("été"), "é".length, /colou?r/.test("the colour"), re);
