// `eval` and `new Function`: code written in the program is translated with it; code
// built while the program runs is evaluated then, with the variables in scope.

const x = 6;
console.log(eval("x * 7"));
console.log(eval("let a = x + 1; a * 2"));

// Declarations made by `eval` belong to the code around it.
eval("var y = 10; function double(n) { return n * 2; }");
console.log(y, double(y));

let count = 0;
eval("count += 5");
console.log(count);

const add = new Function("a", "b", "return a + b");
console.log(add(2, 3));
const greet = Function("name", "return `hi ${name}`");
console.log(greet("ada"));

// Code known only when the program runs.
const n = 4;
const items = [1, 2];
const label = "ok";
const formulas = ["1 + 2 * 3", "`n = ${n}`", "items.length * n", "label.toUpperCase() + '!'"];
for (const code of formulas) console.log(eval(code));
console.log(eval(formulas.join(" + ")));
const body = "return a * b + " + n;
const mul = new Function("a", "b", body);
console.log(mul(3, 5));
const condition = n > 3 ? "n > 3 ? 'big' : 'small'" : "'none'";
console.log(eval(condition), eval("Math.max(n, 10) / 4"), eval("({ n, double: n * 2 })"));
