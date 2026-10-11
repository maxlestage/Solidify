export function shout(s: string): string {
  return s.toUpperCase() + "!";
}

export function repeat(s: string, n: number): string {
  let out = "";
  for (let i = 0; i < n; i++) {
    out += s;
  }
  return out;
}

export const SEPARATOR = " | ";

let calls = 0;

export function counted(s: string): string {
  calls++;
  return `${s} (${calls})`;
}

console.log("strings loaded");
