//! The Solidify website: the home page, the documentation and the playground island. The
//! server (native) renders the pages; the browser (wasm32) runs the playground, where the
//! transpiler runs as WebAssembly, and the motion, the menu and the search.

pub mod catalog;
pub mod examples;
pub mod highlight;
pub mod i18n;
pub mod islands;
pub mod routes;
pub mod search;

// The server's side: the pages, the documentation (built in from `docs/`), the web server.
#[cfg(not(target_arch = "wasm32"))]
pub mod docs;
#[cfg(not(target_arch = "wasm32"))]
pub mod markdown;
#[cfg(not(target_arch = "wasm32"))]
pub mod pages;
#[cfg(not(target_arch = "wasm32"))]
pub mod pwa;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;

#[cfg(target_arch = "wasm32")]
mod client;
