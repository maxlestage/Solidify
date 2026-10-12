//! The site as an installable app: its manifest (built with `active::pwa`), and a service
//! worker written in Rust (`site/worker`) that keeps the pages, the styles, the fonts and
//! the transpiler for offline use. The playground runs Solidify in the browser, so it
//! works without a network too.

use std::path::Path;

use active::pwa::Manifest;

/// The color of the status bar, the splash screen and the icons' background.
pub const THEME: &str = "#0b0b0c";

/// The manifest. Its URLs are relative to it (`manifest.webmanifest` sits at the root of
/// the site), so the base path needs no special care.
pub fn manifest() -> Manifest {
    Manifest::new("Solidify, TypeScript to Rust")
        .short_name("Solidify")
        .description(
            "Write TypeScript or JavaScript, get Rust. The transpiler runs in the browser, offline too.",
        )
        .id("./")
        .start_url("./")
        .scope("./")
        .theme_color(THEME)
        .background_color(THEME)
        .categories(["developer", "productivity", "utilities"])
        .icon("static/icon-192.png", "192x192", "image/png")
        .icon("static/icon-512.png", "512x512", "image/png")
        .maskable_icon("static/icon-maskable-512.png", "512x512", "image/png")
        .icon("static/favicon.svg", "any", "image/svg+xml")
}

/// The service worker's own files, built by `wasm-bindgen --target no-modules` into
/// `site/pkg/worker/`; they are not served on their own.
const WORKER: &str = "pkg/worker";

/// `sw.js`: the loader `wasm-bindgen` generated for the Rust worker, the worker itself
/// inlined (a service worker must register its listeners while its script first runs,
/// so the module cannot be fetched first), and the call that starts it with this
/// version's cache and files. `None` when the worker is not built.
pub fn service_worker(site: &Path, files: &[String], version: &str) -> Option<String> {
    let glue = std::fs::read_to_string(site.join(WORKER).join("solidify_site_worker.js")).ok()?;
    let wasm = std::fs::read(site.join(WORKER).join("solidify_site_worker_bg.wasm")).ok()?;
    let list = files
        .iter()
        .map(|f| json_string(f))
        .collect::<Vec<_>>()
        .join(",");
    Some(format!(
        "// The service worker of the Solidify website: Rust compiled to WebAssembly\n\
         // (site/worker). Below, the loader wasm-bindgen generates, then the module.\n\
         {glue}\n\
         wasm_bindgen.initSync({{module:Uint8Array.from(atob(\"{}\"),c=>c.charCodeAt(0))}});\n\
         wasm_bindgen.serve(\"solidify-{version}\",[{list}]);\n",
        base64(&wasm)
    ))
}

/// The files the pages link to with a version (`?v=` and a hash of the file): a new
/// version of one is a new address, so neither the browser's cache nor an older service
/// worker can serve the previous copy to a newer page.
pub const VERSIONED: [&str; 3] = [
    "static/app.css",
    "pkg/solidify_site.js",
    "pkg/solidify_site_bg.wasm",
];

/// FNV-1a, 64 bits: `hash` mixed with `bytes`.
fn fnv(mut hash: u64, bytes: &[u8]) -> u64 {
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

const FNV_START: u64 = 0xcbf2_9ce4_8422_2325;

/// `file` (relative to `site`) with `?v=` and a hash of its contents, or as it is when it
/// cannot be read.
pub fn versioned(site: &Path, file: &str) -> String {
    match std::fs::read(site.join(file)) {
        Ok(bytes) => format!("{file}?v={:08x}", fnv(FNV_START, &bytes) as u32),
        Err(_) => file.to_string(),
    }
}

/// What the service worker keeps: the pages and every file of `static/` and `pkg/` under
/// `site` (the worker's own files aside, and the [`VERSIONED`] ones at their versioned
/// address), with a version that changes with any of them.
pub fn offline_files(site: &Path, pages: &[(String, String)]) -> (Vec<String>, String) {
    let mut files: Vec<String> = pages.iter().map(|(path, _)| path.clone()).collect();
    let mut hash: u64 = FNV_START;
    let mut mix = |bytes: &[u8]| hash = fnv(hash, bytes);
    for (_, html) in pages {
        mix(html.as_bytes());
    }
    for folder in ["static", "pkg"] {
        let mut stack = vec![site.join(folder)];
        let mut found = Vec::new();
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    found.push(path);
                }
            }
        }
        found.sort();
        for path in found {
            let relative = path
                .strip_prefix(site)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if let Ok(bytes) = std::fs::read(&path) {
                // The worker's files change its script, hence the version, too.
                mix(&bytes);
                if VERSIONED.contains(&relative.as_str()) {
                    files.push(versioned(site, &relative));
                } else if !relative.starts_with(WORKER) {
                    files.push(relative);
                }
            }
        }
    }
    files.push("manifest.webmanifest".into());
    (files, format!("{hash:016x}"))
}

/// A JavaScript string literal.
fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Standard base64, with padding.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0, 255, 254]), "AP/+");
    }

    #[test]
    fn versioned_files_change_address_with_their_contents() {
        let dir = std::env::temp_dir().join(format!("solidify-pwa-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("static")).unwrap();
        std::fs::write(dir.join("static/app.css"), "a{}").unwrap();
        let first = versioned(&dir, "static/app.css");
        std::fs::write(dir.join("static/app.css"), "b{}").unwrap();
        let second = versioned(&dir, "static/app.css");
        assert!(first.starts_with("static/app.css?v="));
        assert_ne!(first, second);
        assert_eq!(versioned(&dir, "pkg/missing.js"), "pkg/missing.js");
        // The worker keeps them at the address the pages use.
        let (files, _) = offline_files(&dir, &[]);
        assert!(files.contains(&second));
        assert!(!files.contains(&"static/app.css".to_string()));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_manifest_names_the_app_and_its_icons() {
        let json = manifest().to_json();
        assert!(json.contains("\"short_name\":\"Solidify\""));
        assert!(json.contains("\"purpose\":\"maskable\""));
    }
}
