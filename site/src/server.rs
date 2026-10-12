//! A dependency-free web server for the site, and the static export used by GitHub Pages.
//!
//! `cargo run -p solidify-site` serves on `PORT` (3000). `cargo run -p solidify-site --
//! export dist` writes the pages and assets to `dist/`. `BASE_PATH` (e.g. `/Solidify`)
//! prefixes every link and `SITE_URL` (e.g. `https://you.github.io/Solidify`) makes the
//! absolute URLs; the files are read from `SITE_DIR` (the `site/` folder by default).

use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use crate::pages;
use crate::routes::{Lang, Route, base, set_base};

/// The `site/` folder: `SITE_DIR`, or the crate's folder.
pub fn site_dir() -> PathBuf {
    std::env::var_os("SITE_DIR")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain; charset=utf-8",
        Some("webmanifest") => "application/manifest+json",
        Some("xml") => "application/xml; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// The public address of the site, with the base path.
fn origin(host: &str) -> String {
    match std::env::var("SITE_URL") {
        Ok(url) if !url.trim().is_empty() => {
            let url = url.trim_end_matches('/').to_string();
            if url.ends_with(base()) {
                url
            } else {
                format!("{url}{}", base())
            }
        }
        _ => format!("http://{host}{}", base()),
    }
}

/// An HTTP response for `path`: status, content type and body.
pub fn respond(path: &str, host: &str) -> (&'static str, &'static str, Vec<u8>) {
    let path = path.split('?').next().unwrap_or("/");
    if path.contains("..") {
        return ("404 Not Found", "text/plain", b"not found".to_vec());
    }
    if let Some(route) = Route::parse(path) {
        return (
            "200 OK",
            "text/html; charset=utf-8",
            pages::render(route, &origin(host)).into_bytes(),
        );
    }
    let relative = path.strip_prefix(base()).unwrap_or(path);
    match relative {
        "/manifest.webmanifest" => {
            return (
                "200 OK",
                content_type(relative),
                crate::pwa::manifest().to_json().into_bytes(),
            );
        }
        "/sitemap.xml" => {
            return (
                "200 OK",
                content_type(relative),
                sitemap(&origin(host)).into_bytes(),
            );
        }
        "/sw.js" => {
            return match service_worker(&origin(host)) {
                Some(worker) => ("200 OK", content_type(relative), worker.into_bytes()),
                None => (
                    "404 Not Found",
                    "text/plain",
                    b"the worker is not built".to_vec(),
                ),
            };
        }
        _ => {}
    }
    let file = match relative {
        "/favicon.svg" => Some(site_dir().join("static/favicon.svg")),
        p if p.starts_with("/static/") || p.starts_with("/pkg/") => Some(site_dir().join(&p[1..])),
        _ => None,
    };
    match file.and_then(|f| std::fs::read(f).ok()) {
        Some(bytes) => ("200 OK", content_type(relative), bytes),
        None => (
            "404 Not Found",
            "text/html; charset=utf-8",
            pages::home(Lang::En, &origin(host)).into_bytes(),
        ),
    }
}

/// `sitemap.xml`: every page, each with its address in the other languages, for search
/// engines.
pub fn sitemap(origin: &str) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
    );
    for route in Route::all() {
        xml.push_str(&format!(
            "  <url>\n    <loc>{origin}{}</loc>\n",
            route.path()
        ));
        for lang in Lang::ALL {
            xml.push_str(&format!(
                "    <xhtml:link rel=\"alternate\" hreflang=\"{}\" href=\"{origin}{}\"/>\n",
                lang.code(),
                route.with_lang(lang).path()
            ));
        }
        xml.push_str("  </url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

/// The service worker for the pages as rendered at `origin` (`None` while
/// `site/pkg/worker` is not built: see site/README.md).
fn service_worker(origin: &str) -> Option<String> {
    let pages: Vec<(String, String)> = Route::all()
        .into_iter()
        .map(|route| {
            let path = match route.path().trim_start_matches('/') {
                "" => "./".to_string(),
                p => p.to_string(),
            };
            (path, pages::render(route, origin))
        })
        .collect();
    let (files, version) = crate::pwa::offline_files(&site_dir(), &pages);
    crate::pwa::service_worker(&site_dir(), &files, &version)
}

/// Serves the site until the process is stopped.
pub fn run() {
    set_base(&std::env::var("BASE_PATH").unwrap_or_default());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).expect("bind");
    println!("http://localhost:{port}{}/", base());
    if !site_dir().join("pkg/solidify_site_bg.wasm").exists() {
        eprintln!(
            "warning: site/pkg is missing, the playground will not be interactive (see site/README.md)"
        );
    }
    for stream in listener.incoming().flatten() {
        std::thread::spawn(move || {
            let mut stream = stream;
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                return;
            }
            let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
            let mut host = String::from("localhost");
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).is_err() || header.trim().is_empty() {
                    break;
                }
                if let Some(value) = header
                    .strip_prefix("Host:")
                    .or_else(|| header.strip_prefix("host:"))
                {
                    host = value.trim().to_string();
                }
            }
            let (status, kind, body) = respond(&path, &host);
            let head = format!(
                "HTTP/1.1 {status}\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        });
    }
}

/// Writes the site as static files into `out`. Returns the number of files written.
pub fn export(out: &Path) -> io::Result<usize> {
    set_base(&std::env::var("BASE_PATH").unwrap_or_default());
    let origin = origin("localhost");
    let mut written = 0;
    let mut write = |path: &str, bytes: &[u8]| -> io::Result<()> {
        let file = out.join(path.trim_start_matches('/'));
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(file, bytes)?;
        written += 1;
        Ok(())
    };
    for route in Route::all() {
        let html = pages::render(route, &origin);
        write(&format!("{}index.html", route.path()), html.as_bytes())?;
    }
    write("404.html", pages::home(Lang::En, &origin).as_bytes())?;
    // The installable app: its manifest, and the service worker that keeps it offline.
    write(
        "manifest.webmanifest",
        crate::pwa::manifest().to_json().as_bytes(),
    )?;
    write("sitemap.xml", sitemap(&origin).as_bytes())?;
    match service_worker(&origin) {
        Some(worker) => write("sw.js", worker.as_bytes())?,
        None => eprintln!("warning: site/pkg/worker is missing, the site will not work offline"),
    }
    let dir = site_dir();
    for folder in ["static", "pkg"] {
        let root = dir.join(folder);
        if !root.exists() {
            if folder == "pkg" {
                eprintln!("warning: site/pkg is missing, the playground will not be interactive");
            }
            continue;
        }
        for file in files_under(&root)? {
            let relative = file.strip_prefix(&dir).unwrap_or(&file);
            // The worker is inlined into sw.js.
            if relative.starts_with("pkg/worker") {
                continue;
            }
            let bytes = std::fs::read(&file)?;
            write(&relative.to_string_lossy().replace('\\', "/"), &bytes)?;
        }
    }
    if let Ok(icon) = std::fs::read(dir.join("static/favicon.svg")) {
        write("favicon.svg", &icon)?;
    }
    // Files are served as they are: no Jekyll processing on GitHub Pages.
    write(".nojekyll", b"")?;
    Ok(written)
}

fn files_under(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            out.extend(files_under(&path)?);
        } else {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_and_files_are_served() {
        let (status, kind, body) = respond("/", "localhost:3000");
        assert_eq!(status, "200 OK");
        assert!(kind.starts_with("text/html"));
        assert!(String::from_utf8_lossy(&body).contains("<html lang=\"en\""));
        let (status, _, body) = respond("/fr/?x=1", "localhost");
        assert_eq!(status, "200 OK");
        assert!(String::from_utf8_lossy(&body).contains("<html lang=\"fr\""));
        let (status, kind, _) = respond("/static/app.css", "localhost");
        assert_eq!(status, "200 OK");
        assert_eq!(kind, "text/css; charset=utf-8");
        let (status, _, body) = respond("/es/docs/cli/", "localhost");
        assert_eq!(status, "200 OK");
        assert!(String::from_utf8_lossy(&body).contains("<html lang=\"es\""));
        let (status, _, _) = respond("/static/../Cargo.toml", "localhost");
        assert_eq!(status, "404 Not Found");
    }

    /// Nothing scrolls sideways (site/README.md, "No horizontal scrolling"): no rule of
    /// the stylesheet makes an element scroll horizontally, or keeps code from wrapping.
    #[test]
    fn nothing_scrolls_sideways() {
        let css = std::fs::read_to_string(site_dir().join("static/app.css")).unwrap();
        for (n, line) in css.lines().enumerate() {
            let rule = line.trim();
            for forbidden in [
                "overflow: auto",
                "overflow: scroll",
                "overflow-x: auto",
                "overflow-x: scroll",
                "white-space: pre;",
            ] {
                assert!(
                    !rule.starts_with(forbidden),
                    "app.css:{}: `{rule}` can scroll sideways",
                    n + 1
                );
            }
        }
        assert!(css.contains("overflow-wrap: anywhere"));
    }

    #[test]
    fn export_writes_every_page() {
        let dir = std::env::temp_dir().join(format!("solidify-site-{}", std::process::id()));
        let n = export(&dir).unwrap();
        assert!(n >= 4);
        assert!(dir.join("index.html").exists());
        assert!(dir.join("fr/index.html").exists());
        assert!(dir.join("docs/index.html").exists());
        assert!(dir.join("es/docs/cli/index.html").exists());
        assert!(dir.join("static/app.css").exists());
        assert!(dir.join(".nojekyll").exists());
        assert!(dir.join("manifest.webmanifest").exists());
        let sitemap = std::fs::read_to_string(dir.join("sitemap.xml")).unwrap();
        assert_eq!(sitemap.matches("<url>").count(), Route::all().len());
        assert!(sitemap.contains("/fr/docs/quick-start/</loc>"));
        // The worker exists once site/pkg/worker is built (CI builds it before exporting).
        if let Ok(sw) = std::fs::read_to_string(dir.join("sw.js")) {
            assert!(sw.contains("\"static/app.css\"") && sw.contains("\"fr/\""));
            assert!(sw.contains("\"fr/docs/quick-start/\""));
            assert!(sw.contains("wasm_bindgen.serve(\"solidify-"));
            assert!(!sw.contains("\"pkg/worker"));
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
