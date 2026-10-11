// Node's `fs` module: files as text, synchronous, with callbacks, and with promises.

export function existsSync(path: string): boolean {
  return __fs_exists(path);
}

export function readFileSync(path: string, encoding?: string): string {
  if (!__fs_exists(path)) {
    throw new Error(`ENOENT: no such file or directory, open '${path}'`);
  }
  if (__fs_is_dir(path)) {
    throw new Error("EISDIR: illegal operation on a directory, read");
  }
  return __fs_read(path);
}

export function writeFileSync(path: string, data: string): void {
  __fs_write(path, data);
}

export function appendFileSync(path: string, data: string): void {
  __fs_append(path, data);
}

export function readdirSync(path: string): string[] {
  if (!__fs_is_dir(path)) {
    throw new Error(`ENOENT: no such file or directory, scandir '${path}'`);
  }
  return __fs_readdir(path);
}

export interface MkdirOptions {
  recursive?: boolean;
}

export function mkdirSync(path: string, options?: MkdirOptions): void {
  const recursive = options !== undefined && options.recursive === true;
  if (!recursive && __fs_exists(path)) {
    throw new Error(`EEXIST: file already exists, mkdir '${path}'`);
  }
  __fs_mkdir(path, recursive);
}

export interface RmOptions {
  recursive?: boolean;
  force?: boolean;
}

export function rmSync(path: string, options?: RmOptions): void {
  const force = options !== undefined && options.force === true;
  if (!__fs_exists(path)) {
    if (force) {
      return;
    }
    throw new Error(`ENOENT: no such file or directory, lstat '${path}'`);
  }
  __fs_remove(path, options !== undefined && options.recursive === true);
}

export function unlinkSync(path: string): void {
  if (!__fs_exists(path)) {
    throw new Error(`ENOENT: no such file or directory, unlink '${path}'`);
  }
  __fs_remove(path, false);
}

export function renameSync(from: string, to: string): void {
  __fs_rename(from, to);
}

export function copyFileSync(from: string, to: string): void {
  if (!__fs_exists(from)) {
    throw new Error(`ENOENT: no such file or directory, copyfile '${from}' -> '${to}'`);
  }
  __fs_copy(from, to);
}

export class Stats {
  constructor(
    public size: number,
    private dir: boolean,
  ) {}
  isFile(): boolean {
    return !this.dir;
  }
  isDirectory(): boolean {
    return this.dir;
  }
}

export function statSync(path: string): Stats {
  if (!__fs_exists(path)) {
    throw new Error(`ENOENT: no such file or directory, stat '${path}'`);
  }
  return new Stats(__fs_size(path), __fs_is_dir(path));
}

// Callbacks run after the current code, like Node's.
export function readFile(
  path: string,
  encoding: string,
  callback: (err: Error | null, data: string) => void,
): void {
  setTimeout(() => {
    if (!__fs_exists(path)) {
      callback(new Error(`ENOENT: no such file or directory, open '${path}'`), "");
    } else {
      callback(null, __fs_read(path));
    }
  }, 0);
}

export function writeFile(path: string, data: string, callback: (err: Error | null) => void): void {
  setTimeout(() => {
    __fs_write(path, data);
    callback(null);
  }, 0);
}
