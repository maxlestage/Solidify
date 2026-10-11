// Error classes of the program, switch cases that fall through, break and continue
// inside try.

class AppError extends Error {
  constructor(message: string, public code: number) {
    super(message);
    this.name = "AppError";
  }
}
class NotFound extends AppError {
  constructor(public key: string) {
    super(`missing ${key}`, 404);
    this.name = "NotFound";
  }
}
class Plain extends Error {}
function find(key: string): number {
  if (key === "a") return 1;
  if (key === "boom") throw new Error("plain failure");
  throw new NotFound(key);
}
for (const k of ["a", "b", "boom"]) {
  try {
    console.log("found", find(k));
  } catch (e) {
    if (e instanceof NotFound) console.log("not found:", e.key, e.code, e.message, e.name);
    else if (e instanceof AppError) console.log("app", e.code);
    else console.log("other:", (e as Error).message, e instanceof NotFound);
  }
}
try {
  throw new Plain("p");
} catch (e) {
  console.log(e instanceof Plain, e instanceof AppError, (e as Error).message, (e as Error).name);
}

function kind(n: number): string {
  switch (n) { case 1: case 2: return "small"; case 3: console.log("three"); case 4: return "mid"; default: return "big"; }
}
console.log(kind(1), kind(3), kind(9));

for (let i = 0; i < 5; i++) {
  try { if (i === 1) continue; if (i === 3) break; console.log("i", i); } finally { console.log("f", i); }
}
