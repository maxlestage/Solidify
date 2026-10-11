//! The Solidify website: pages and the playground island, shared by the server (native)
//! and the browser (wasm32), where the transpiler runs as WebAssembly.

pub mod examples;
pub mod highlight;
pub mod i18n;
pub mod islands;
pub mod pages;
pub mod routes;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

#[cfg(target_arch = "wasm32")]
mod client;
