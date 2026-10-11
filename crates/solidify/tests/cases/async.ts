async function fetchValue(x: number): Promise<number> {
  return x * 2;
}

async function total(xs: number[]): Promise<number> {
  let t = 0;
  for (const x of xs) {
    t += await fetchValue(x);
  }
  return t;
}

const result = await total([1, 2, 3]);
console.log("total:", result);
