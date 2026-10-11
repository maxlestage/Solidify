// Node's `os` module.

export const EOL = "\n";

export function platform(): string {
  return __os_info("platform");
}

export function arch(): string {
  return __os_info("arch");
}

export function type(): string {
  return __os_info("type");
}

export function homedir(): string {
  return __os_info("homedir");
}

export function tmpdir(): string {
  return __os_info("tmpdir");
}

export function hostname(): string {
  return __os_info("hostname");
}

export function availableParallelism(): number {
  return __os_cpus();
}
