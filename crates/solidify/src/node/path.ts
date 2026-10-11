// Node's `path` module (POSIX), written in the subset Solidify translates.

export const sep = "/";
export const delimiter = ":";

export function isAbsolute(p: string): boolean {
  return p.startsWith("/");
}

export function normalize(p: string): string {
  if (p === "") {
    return ".";
  }
  const absolute = p.startsWith("/");
  const trailing = p.endsWith("/");
  const out: string[] = [];
  for (const part of p.split("/")) {
    if (part === "" || part === ".") {
      continue;
    }
    if (part === "..") {
      if (out.length > 0 && out[out.length - 1] !== "..") {
        out.pop();
      } else if (!absolute) {
        out.push("..");
      }
      continue;
    }
    out.push(part);
  }
  let result = out.join("/");
  if (absolute) {
    result = "/" + result;
  }
  if (result === "") {
    return absolute ? "/" : ".";
  }
  if (trailing && result !== "/") {
    result += "/";
  }
  return result;
}

export function join(...parts: string[]): string {
  const kept = parts.filter((p) => p !== "");
  if (kept.length === 0) {
    return ".";
  }
  return normalize(kept.join("/"));
}

export function resolve(...parts: string[]): string {
  let result = "";
  for (let i = parts.length - 1; i >= 0; i--) {
    const p = parts[i];
    if (p === "") {
      continue;
    }
    result = result === "" ? p : p + "/" + result;
    if (p.startsWith("/")) {
      break;
    }
  }
  if (!result.startsWith("/")) {
    result = result === "" ? __os_info("cwd") : __os_info("cwd") + "/" + result;
  }
  const n = normalize(result);
  return n.length > 1 && n.endsWith("/") ? n.slice(0, -1) : n;
}

export function dirname(p: string): string {
  if (p === "") {
    return ".";
  }
  let end = p.length;
  while (end > 1 && p[end - 1] === "/") {
    end--;
  }
  const i = p.lastIndexOf("/", end - 1);
  if (i < 0) {
    return ".";
  }
  if (i === 0) {
    return "/";
  }
  return p.slice(0, i);
}

export function basename(p: string, ext?: string): string {
  let end = p.length;
  while (end > 1 && p[end - 1] === "/") {
    end--;
  }
  const trimmed = p.slice(0, end);
  const i = trimmed.lastIndexOf("/");
  const base = i >= 0 ? trimmed.slice(i + 1) : trimmed;
  if (ext !== undefined && ext !== base && base.endsWith(ext)) {
    return base.slice(0, base.length - ext.length);
  }
  return base;
}

export function extname(p: string): string {
  const base = basename(p);
  const i = base.lastIndexOf(".");
  if (i <= 0) {
    return "";
  }
  return base.slice(i);
}

export function relative(from: string, to: string): string {
  const a = resolve(from).split("/").filter((x) => x !== "");
  const b = resolve(to).split("/").filter((x) => x !== "");
  let common = 0;
  while (common < a.length && common < b.length && a[common] === b[common]) {
    common++;
  }
  const up: string[] = [];
  for (let i = common; i < a.length; i++) {
    up.push("..");
  }
  return up.concat(b.slice(common)).join("/");
}

export interface ParsedPath {
  root: string;
  dir: string;
  base: string;
  ext: string;
  name: string;
}

export function parse(p: string): ParsedPath {
  const base = basename(p);
  const ext = extname(p);
  const dir = p.includes("/") ? dirname(p) : "";
  return {
    root: p.startsWith("/") ? "/" : "",
    dir,
    base,
    ext,
    name: ext === "" ? base : base.slice(0, base.length - ext.length),
  };
}

export function format(p: ParsedPath): string {
  const base = p.base !== "" ? p.base : p.name + p.ext;
  if (p.dir === "") {
    return p.root + base;
  }
  return p.dir === p.root ? p.dir + base : p.dir + "/" + base;
}
