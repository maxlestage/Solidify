//! The site as an installable app: its manifest, and a service worker that keeps the
//! pages, the styles, the fonts and the transpiler (WebAssembly) for offline use. The
//! playground runs Solidify in the browser, so it works without a network too.

use std::path::Path;

/// `manifest.webmanifest`: the app's name, colors and icons. Its URLs are relative to it,
/// so the base path needs no special care.
pub fn manifest() -> String {
    r##"{
  "name": "Solidify, TypeScript to Rust",
  "short_name": "Solidify",
  "description": "Write TypeScript or JavaScript, get Rust. The transpiler runs in the browser, offline too.",
  "id": "./",
  "start_url": "./",
  "scope": "./",
  "display": "standalone",
  "background_color": "#0b0b0c",
  "theme_color": "#0b0b0c",
  "categories": ["developer", "productivity", "utilities"],
  "icons": [
    { "src": "static/icon-192.png", "sizes": "192x192", "type": "image/png" },
    { "src": "static/icon-512.png", "sizes": "512x512", "type": "image/png" },
    { "src": "static/icon-maskable-512.png", "sizes": "512x512", "type": "image/png", "purpose": "maskable" },
    { "src": "static/favicon.svg", "sizes": "any", "type": "image/svg+xml" }
  ],
  "shortcuts": [
    { "name": "Playground", "url": "./#playground" },
    { "name": "Bac à sable", "url": "fr/#playground" },
    { "name": "Playground (ES)", "url": "es/#playground" }
  ]
}
"##
    .to_string()
}

/// `sw.js`, kept next to the pages: it stores `files` (relative to it) when installed, and
/// takes a new name whenever `version` changes, so a new site replaces the old one.
pub fn service_worker(files: &[String], version: &str) -> String {
    let list = files
        .iter()
        .map(|f| format!("  \"{f}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        r#"// Solidify's service worker: the site works offline once visited.
const CACHE = "solidify-{version}";
const FILES = [
{list}
];

self.addEventListener("install", (event) => {{
  event.waitUntil(
    caches.open(CACHE).then((cache) => cache.addAll(FILES)).then(() => self.skipWaiting())
  );
}});

// The previous versions are removed once this one runs.
self.addEventListener("activate", (event) => {{
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter((key) => key.startsWith("solidify-") && key !== CACHE)
            .map((key) => caches.delete(key))
        )
      )
      .then(() => self.clients.claim())
  );
}});

self.addEventListener("fetch", (event) => {{
  const request = event.request;
  if (request.method !== "GET" || new URL(request.url).origin !== self.location.origin) {{
    return;
  }}
  // Pages: the network first, the stored copy offline.
  if (request.mode === "navigate") {{
    event.respondWith(
      fetch(request)
        .then((response) => {{
          const copy = response.clone();
          caches.open(CACHE).then((cache) => cache.put(request, copy));
          return response;
        }})
        .catch(() =>
          caches.match(request).then((hit) => hit || caches.match(new URL("./", self.location)))
        )
    );
    return;
  }}
  // Styles, fonts, icons, the transpiler: stored with this version.
  event.respondWith(
    caches.match(request).then(
      (hit) =>
        hit ||
        fetch(request).then((response) => {{
          if (response.ok) {{
            const copy = response.clone();
            caches.open(CACHE).then((cache) => cache.put(request, copy));
          }}
          return response;
        }})
    )
  );
}});
"#
    )
}

/// What the service worker stores: the pages and every file of `static/` and `pkg/`
/// under `site`, with a version that changes with any of them.
pub fn offline_files(site: &Path, pages: &[(String, String)]) -> (Vec<String>, String) {
    let mut files: Vec<String> = pages.iter().map(|(path, _)| path.clone()).collect();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |bytes: &[u8]| {
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
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
            if let Ok(bytes) = std::fs::read(&path) {
                mix(&bytes);
                let relative = path.strip_prefix(site).unwrap_or(&path);
                files.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    files.push("manifest.webmanifest".into());
    (files, format!("{hash:016x}"))
}

/// Registers the service worker, and shows the install button when the browser offers to
/// install the app.
pub fn register_script(sw: &str) -> String {
    format!(
        "<script>\
if(\"serviceWorker\" in navigator){{addEventListener(\"load\",function(){{navigator.serviceWorker.register(\"{sw}\").catch(function(){{}})}})}}\
var installPrompt=null;\
addEventListener(\"beforeinstallprompt\",function(e){{e.preventDefault();installPrompt=e;var b=document.getElementById(\"install-app\");if(b){{b.hidden=false}}}});\
addEventListener(\"appinstalled\",function(){{var b=document.getElementById(\"install-app\");if(b){{b.hidden=true}}}});\
addEventListener(\"click\",function(e){{if(e.target&&e.target.id===\"install-app\"&&installPrompt){{installPrompt.prompt();installPrompt=null;e.target.hidden=true}}}});\
</script>"
    )
}
