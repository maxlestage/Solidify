/// The native side of Node's `fs` and `os` modules.
#[allow(dead_code)]
fn js_fs_fail(op: &str, path: &str, e: std::io::Error) -> ! {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => "ENOENT: no such file or directory",
        std::io::ErrorKind::PermissionDenied => "EACCES: permission denied",
        std::io::ErrorKind::AlreadyExists => "EEXIST: file already exists",
        _ => "EIO: i/o error",
    };
    panic!("Error: {code}, {op} '{path}'")
}

#[allow(dead_code)]
fn js_node_fs_read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| js_fs_fail("open", path, e))
}

#[allow(dead_code)]
fn js_node_fs_write(path: &str, data: &str) {
    std::fs::write(path, data).unwrap_or_else(|e| js_fs_fail("open", path, e))
}

#[allow(dead_code)]
fn js_node_fs_append(path: &str, data: &str) {
    use std::io::Write;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| f.write_all(data.as_bytes()))
        .unwrap_or_else(|e| js_fs_fail("open", path, e))
}

#[allow(dead_code)]
fn js_node_fs_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

#[allow(dead_code)]
fn js_node_fs_is_dir(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

#[allow(dead_code)]
fn js_node_fs_size(path: &str) -> i64 {
    std::fs::metadata(path).map_or(0, |m| m.len() as i64)
}

/// The names in a directory, sorted like Node gives them.
#[allow(dead_code)]
fn js_node_fs_readdir(path: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(path)
        .unwrap_or_else(|e| js_fs_fail("scandir", path, e))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[allow(dead_code)]
fn js_node_fs_mkdir(path: &str, recursive: bool) {
    let r = if recursive {
        std::fs::create_dir_all(path)
    } else {
        std::fs::create_dir(path)
    };
    r.unwrap_or_else(|e| js_fs_fail("mkdir", path, e))
}

#[allow(dead_code)]
fn js_node_fs_remove(path: &str, recursive: bool) {
    let p = std::path::Path::new(path);
    let r = if p.is_dir() {
        if recursive {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_dir(p)
        }
    } else {
        std::fs::remove_file(p)
    };
    r.unwrap_or_else(|e| js_fs_fail("rm", path, e))
}

#[allow(dead_code)]
fn js_node_fs_rename(from: &str, to: &str) {
    std::fs::rename(from, to).unwrap_or_else(|e| js_fs_fail("rename", from, e))
}

#[allow(dead_code)]
fn js_node_fs_copy(from: &str, to: &str) {
    std::fs::copy(from, to)
        .map(|_| ())
        .unwrap_or_else(|e| js_fs_fail("copyfile", from, e))
}

/// `os.platform()`, `os.homedir()`, `process.cwd()`…
#[allow(dead_code)]
fn js_node_os_info(key: &str) -> String {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    match key {
        "platform" => match std::env::consts::OS {
            "macos" => "darwin".to_string(),
            "windows" => "win32".to_string(),
            other => other.to_string(),
        },
        "arch" => match std::env::consts::ARCH {
            "x86_64" => "x64".to_string(),
            "aarch64" => "arm64".to_string(),
            "x86" => "ia32".to_string(),
            other => other.to_string(),
        },
        "type" => match std::env::consts::OS {
            "linux" => "Linux".to_string(),
            "macos" => "Darwin".to_string(),
            "windows" => "Windows_NT".to_string(),
            other => other.to_string(),
        },
        "homedir" => env("HOME"),
        "tmpdir" => std::env::temp_dir()
            .to_string_lossy()
            .trim_end_matches('/')
            .to_string(),
        "hostname" => std::fs::read_to_string("/etc/hostname")
            .map(|h| h.trim().to_string())
            .unwrap_or_else(|_| env("HOSTNAME")),
        "cwd" => std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

#[allow(dead_code)]
fn js_node_os_cpus() -> i64 {
    std::thread::available_parallelism().map_or(1, |n| n.get() as i64)
}
