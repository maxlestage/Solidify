// Node's `fs/promises` module.

export async function readFile(path: string, encoding?: string): Promise<string> {
  if (!__fs_exists(path)) {
    throw new Error(`ENOENT: no such file or directory, open '${path}'`);
  }
  return __fs_read(path);
}

export async function writeFile(path: string, data: string): Promise<void> {
  __fs_write(path, data);
}

export async function appendFile(path: string, data: string): Promise<void> {
  __fs_append(path, data);
}

export async function readdir(path: string): Promise<string[]> {
  if (!__fs_is_dir(path)) {
    throw new Error(`ENOENT: no such file or directory, scandir '${path}'`);
  }
  return __fs_readdir(path);
}

export async function mkdir(path: string): Promise<void> {
  __fs_mkdir(path, true);
}

export async function rm(path: string): Promise<void> {
  if (__fs_exists(path)) {
    __fs_remove(path, true);
  }
}
