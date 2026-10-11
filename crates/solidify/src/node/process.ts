// Node's `process` module: the global `process`, its parts as names.

export const argv = process.argv;
export const env = process.env;
export const platform = process.platform;

export function cwd(): string {
  return process.cwd();
}

export function exit(code?: number): void {
  process.exit(code ?? 0);
}
