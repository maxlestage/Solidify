const words = ["pear", "apple", "fig", "banana"];
const lengths = words.map((w) => w.length);
const long = words.filter((w) => w.length > 3);
const total = lengths.reduce((acc, n) => acc + n, 0);
console.log(lengths.join(","), long.join(" "), total);
const sorted = [...words];
sorted.sort((a, b) => a.localeCompare(b));
console.log(sorted.join(" "));
const nums = [5, 3, 9, 1];
nums.push(7);
nums.sort((a, b) => a - b);
console.log(nums.join(" "), nums.includes(9), nums.indexOf(3), Math.max(...nums));
let found = words.find((w) => w.startsWith("b"));
if (found !== undefined) {
  console.log(found.toUpperCase());
}
