# The whole site from Rust sources: the WebAssembly bundle, the service worker (Rust
# too), then the server.

FROM rust:1-slim-bookworm AS build
RUN rustup target add wasm32-unknown-unknown \
 && cargo install wasm-bindgen-cli --version 0.2.129 --locked
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
# The documentation's pages are built into the server.
COPY docs docs
COPY site site
RUN cargo build -p solidify-site --lib --target wasm32-unknown-unknown --profile wasm-release \
 && wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/solidify_site.wasm \
 && cargo build -p solidify-site-worker --target wasm32-unknown-unknown --profile wasm-release \
 && wasm-bindgen --target no-modules --no-typescript --out-dir site/pkg/worker target/wasm32-unknown-unknown/wasm-release/solidify_site_worker.wasm \
 && cargo build -p solidify-site --bin solidify-site --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /src/target/release/solidify-site /app/solidify-site
COPY --from=build /src/site/static /app/site/static
COPY --from=build /src/site/pkg /app/site/pkg
ENV SITE_DIR=/app/site
# PORT is read from the environment; 3000 otherwise.
CMD ["/app/solidify-site"]
