function wait(ms: number): Promise<number> { return new Promise((r) => setTimeout(() => r(ms), ms)); }
async function job(name: string, ms: number) { await wait(ms); console.log(name); }
job("slow", 30);
job("fast", 5);
console.log("sync");
