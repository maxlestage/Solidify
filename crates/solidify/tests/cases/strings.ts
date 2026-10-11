const s = "  Hello, World  ";
const t = s.trim();
console.log(t.length, t.toUpperCase(), t.toLowerCase());
console.log(t.slice(0, 5), t.slice(-5), t.substring(7), t.charAt(1), t[4]);
console.log(t.indexOf("World"), t.indexOf("x"), t.includes("lo,"), t.startsWith("He"), t.endsWith("!"));
console.log(t.split(", ").join(" & "), "a-b-c".split("-").length, "abc".split("").join("|"));
console.log("ab".repeat(3), "x".padStart(3, "0"), "5".padEnd(3, "."), "a.b.c".replace(".", "/"), "a.b.c".replaceAll(".", "/"));
let acc = "";
for (const ch of "abc") {
  acc += ch.toUpperCase();
  if (ch === "b") acc += "!";
}
console.log(acc, acc.length);
const n = 3.14159;
console.log(n.toFixed(2), (10).toString(), String(42) + "!", "7" + 7, 7 + "7");
console.log(parseInt("42px"), parseFloat("2.5kg"), Number("12") + 1, Number("x"));
const words = ["b", "a", "c"];
words.sort();
console.log(words.join(""), "b" < "c", "Zed" > "apple");
