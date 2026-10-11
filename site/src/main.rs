//! `cargo run -p solidify-site`: the web server (build the browser bundle first, see
//! site/README.md). `cargo run -p solidify-site -- export dist`: the site as static files.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("export") => {
            let dir = args.next().unwrap_or_else(|| "dist".into());
            match solidify_site::server::export(std::path::Path::new(&dir)) {
                Ok(files) => println!("{files} files written to {dir}"),
                Err(error) => {
                    eprintln!("export failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some(other) => {
            eprintln!("unknown command {other:?}: run without arguments, or `export <dir>`");
            std::process::exit(2);
        }
        None => solidify_site::server::run(),
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}
