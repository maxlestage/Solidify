//! The service worker of the Solidify website, written in Rust and compiled to
//! WebAssembly: the site keeps working offline, playground included.
//!
//! The server writes `sw.js` (see `site/src/pwa.rs`): the code `wasm-bindgen` generates
//! to load a WebAssembly module, this module inlined, and a call to [`serve`] with the
//! name of this version's cache and the files to keep. Browsers only start service
//! workers from a script, and only listen to the events a worker registers while that
//! script first runs: [`serve`] registers them at once.
//!
//! Strategy:
//!
//! - **install**: every file of the list is downloaded into the cache of this version;
//! - **activate**: the caches of older versions are deleted, open pages are taken over;
//! - **pages**: the network first (the latest version), the saved copy offline, else the
//!   home page;
//! - **other files** of the site: the saved copy, else the network (and saved);
//! - other sites, and anything but `GET`: left to the browser.

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use web_sys::{
    Cache, ExtendableEvent, FetchEvent, Request, RequestMode, Response, ResponseType,
    ServiceWorkerGlobalScope,
};

/// The worker's global scope.
fn scope() -> ServiceWorkerGlobalScope {
    js_sys::global().unchecked_into()
}

/// The prefix of every cache this site creates: older versions are found by it.
const PREFIX: &str = "solidify-";

/// Starts the worker: `cache` names this version's cache (`solidify-` and a hash of the
/// site), `files` are the URLs to keep, relative to the worker.
#[wasm_bindgen]
pub fn serve(cache: String, files: Vec<String>) {
    let global = scope();

    let name = cache.clone();
    listen(&global, "install", move |event: ExtendableEvent| {
        let (name, files) = (name.clone(), files.clone());
        let _ = event.wait_until(&future_to_promise(async move {
            let cache = open(&name).await?;
            let list: js_sys::Array = files.iter().map(|f| JsValue::from_str(f)).collect();
            JsFuture::from(cache.add_all_with_str_sequence(&list)).await?;
            JsFuture::from(scope().skip_waiting()?).await?;
            Ok(JsValue::UNDEFINED)
        }));
    });

    let name = cache.clone();
    listen(&global, "activate", move |event: ExtendableEvent| {
        let name = name.clone();
        let _ = event.wait_until(&future_to_promise(async move {
            let caches = scope().caches()?;
            let keys: js_sys::Array = JsFuture::from(caches.keys()).await?.unchecked_into();
            for key in keys.iter().filter_map(|k| k.as_string()) {
                if key.starts_with(PREFIX) && key != name {
                    JsFuture::from(caches.delete(&key)).await?;
                }
            }
            JsFuture::from(scope().clients().claim()).await?;
            Ok(JsValue::UNDEFINED)
        }));
    });

    listen(&global, "fetch", move |event: FetchEvent| {
        let request = event.request();
        if request.method() != "GET" || !same_origin(&request.url()) {
            return;
        }
        let answer = if request.mode() == RequestMode::Navigate {
            future_to_promise(page(request, cache.clone()))
        } else {
            future_to_promise(file(request, cache.clone()))
        };
        let _ = event.respond_with(&answer);
    });
}

/// Calls `f` with every `event` sent to the worker.
fn listen<E: JsCast + 'static>(
    global: &ServiceWorkerGlobalScope,
    event: &str,
    f: impl Fn(E) + 'static,
) {
    let closure = Closure::<dyn Fn(web_sys::Event)>::new(move |e: web_sys::Event| {
        f(e.unchecked_into());
    });
    let _ = global.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref());
    // The worker lives as long as its listeners.
    closure.forget();
}

/// Whether `url` belongs to this site.
fn same_origin(url: &str) -> bool {
    let origin = scope().location().origin();
    url.starts_with(&origin)
        && matches!(
            url.as_bytes().get(origin.len()),
            None | Some(b'/' | b'?' | b'#')
        )
}

async fn open(name: &str) -> Result<Cache, JsValue> {
    let caches = scope().caches()?;
    Ok(JsFuture::from(caches.open(name)).await?.unchecked_into())
}

/// The saved copy of `request`, if any.
async fn saved(request: &Request) -> Result<Option<Response>, JsValue> {
    let caches = scope().caches()?;
    let hit = JsFuture::from(caches.match_with_request(request)).await?;
    Ok(hit.dyn_into::<Response>().ok())
}

/// Fetches `request` from the network and keeps a copy of a good answer.
async fn network(request: &Request, cache: &str) -> Result<Response, JsValue> {
    let response: Response = JsFuture::from(scope().fetch_with_request(request))
        .await?
        .unchecked_into();
    if response.ok() && response.type_() == ResponseType::Basic {
        let copy = response.clone()?;
        let cache = open(cache).await?;
        JsFuture::from(cache.put_with_request(request, &copy)).await?;
    }
    Ok(response)
}

/// A page: the network first, the saved copy offline, else the home page.
async fn page(request: Request, cache: String) -> Result<JsValue, JsValue> {
    match network(&request, &cache).await {
        Ok(response) => Ok(response.into()),
        Err(_) => {
            if let Some(hit) = saved(&request).await? {
                return Ok(hit.into());
            }
            let home = Request::new_with_str("./")?;
            match saved(&home).await? {
                Some(hit) => Ok(hit.into()),
                None => Ok(Response::error().into()),
            }
        }
    }
}

/// Any other file of the site: the saved copy, else the network.
async fn file(request: Request, cache: String) -> Result<JsValue, JsValue> {
    if let Some(hit) = saved(&request).await? {
        return Ok(hit.into());
    }
    Ok(network(&request, &cache).await?.into())
}
