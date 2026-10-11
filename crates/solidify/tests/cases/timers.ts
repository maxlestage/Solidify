// Timers and promises run on an event loop once the script is done, like in Node.
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function step(name: string, ms: number): Promise<string> {
  await sleep(ms);
  console.log("done", name);
  return name.toUpperCase();
}

function makeCounter(): () => number {
  let count = 0;
  return () => {
    count += 1;
    return count;
  };
}

const log: string[] = [];
setTimeout(() => log.push("late"), 200);
setTimeout(() => {
  log.push("soon");
}, 50);
let ticks = 0;
const timer = setInterval(() => {
  ticks++;
  log.push(`tick ${ticks}`);
  if (ticks === 3) {
    clearInterval(timer);
    console.log("ticks:", ticks, log.join(", "));
  }
}, 20);
const first = await step("a", 5);
const second = await step("b", 1);
console.log(first, second);
const value = await new Promise<number>((resolve) => {
  setTimeout(() => resolve(42), 100);
});
console.log("value", value, log.length);
const next = makeCounter();
next();
next();
console.log("counter", next());
const all = await Promise.all([step("c", 2), step("d", 3)]);
console.log(all.join("+"));
