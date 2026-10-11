// Plain JavaScript: no annotations at all.
function greet(name) {
  return "Hello, " + name + "!";
}

function average(values) {
  let total = 0;
  for (const v of values) {
    total += v;
  }
  return total / values.length;
}

function largest(items) {
  let best = items[0];
  for (const item of items) {
    if (item > best) best = item;
  }
  return best;
}

const scores = [12, 7, 19, 3];
console.log(greet("Ada"), average(scores), largest(scores), largest(["pear", "zebra", "apple"]));
let count = 0;
while (count < 3) {
  count++;
}
const labels = scores.map((s) => (s > 10 ? "high" : "low"));
console.log(count, labels.join(","), scores.filter((s) => s % 2 === 1).length);
