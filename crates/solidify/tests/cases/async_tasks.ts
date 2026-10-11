async function first<T>(items: T[]): Promise<T> {
  await null;
  return items[0];
}
async function loop(): Promise<number> {
  let sum = 0;
  for (let i = 0; i < 3; i++) {
    sum += await first([i, 10]);
  }
  return sum;
}
const delays = [3, 1, 2];
async function ordered() {
  const out: string[] = [];
  await Promise.all(delays.map(async (d) => {
    await new Promise<void>((r) => setTimeout(r, d * 5));
    out.push(`d${d}`);
  }));
  console.log(out.join(","));
}
async function fails(): Promise<string> {
  throw new Error("boom");
}
async function outer(): Promise<string> {
  return fails();
}
let counter = 0;
const tick = async () => {
  counter++;
  return counter;
};
async function run() {
  console.log(await first(["a", "b"]), await loop());
  await ordered();
  try {
    await outer();
  } catch (err) {
    console.log("outer failed:", (err as Error).message);
  }
  await tick();
  await tick();
  console.log("counter", counter);
  const winner = await Promise.race([
    new Promise<string>((r) => setTimeout(() => r("slow"), 20)),
    new Promise<string>((r) => setTimeout(() => r("fast"), 5)),
  ]);
  console.log(winner);
  const any = await Promise.any([Promise.reject(new Error("x")), Promise.resolve(7)]);
  console.log(any);
  const [n, s] = await Promise.all([Promise.resolve(1), first(["z"])]);
  console.log(n, s);
}
run().then(() => console.log("all done"));
