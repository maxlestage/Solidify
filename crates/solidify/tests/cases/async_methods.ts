function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
class Account {
  balance = 0;
  constructor(public owner: string) {}
  async deposit(n: number): Promise<number> {
    await sleep(2);
    this.balance += n;
    return this.balance;
  }
}
async function risky(n: number): Promise<number> {
  await sleep(1);
  if (n > 2) throw new Error(`too big: ${n}`);
  return n * 10;
}
async function main() {
  const acc = new Account("ann");
  await acc.deposit(5);
  const total = await acc.deposit(7);
  console.log(acc.owner, total, acc.balance);
  try {
    await risky(5);
  } catch (e) {
    console.log("caught:", (e as Error).message);
  }
  const p = risky(1);
  console.log("started");
  console.log(await p, await p);
  const doubled = await Promise.all([1, 2, 3].map(async (x) => {
    await sleep(3 - x);
    console.log("done", x);
    return x * 2;
  }));
  console.log(doubled);
  const results = await Promise.allSettled([risky(1), risky(9)]);
  for (const r of results) console.log(r.status, r.value, r.reason?.message);
  risky(1).then((v) => risky(v)).catch((e) => console.log("chain:", e.message)).finally(() => console.log("finally"));
}
Promise.resolve(1).then((v) => console.log("micro", v));
console.log("sync first");
main();
console.log("after main call");
