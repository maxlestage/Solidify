# Instalación

No hay nada que instalar salvo Node.js: `npx` ejecuta Solidify, compilado a WebAssembly, sobre tus archivos. También puedes usarlo en el navegador, sin conexión, o como biblioteca Rust.

## Con npx

Necesitas Node.js 20 o posterior. Después, en la carpeta de tu programa:

```sh
npx solidify-rs main.ts -o app.rs
```

La primera ejecución descarga el paquete `solidify-rs` (unos 2 MB, el propio transpilador); las siguientes arrancan al instante. Si prefieres tenerlo en un proyecto:

```sh
npm install --save-dev solidify-rs
npx solidify-rs main.ts -o app.rs
```

La página de la [línea de comandos](cli.md) enumera todas las opciones.

## Compilar la salida

La salida es un único archivo Rust para la edición 2024. Instala Rust con [rustup](https://rustup.rs) y después:

```sh
rustc --edition 2024 -O app.rs -o app
./app
```

No necesita ningún crate: las pocas funciones auxiliares que usa un programa (imprimir números como lo hace JavaScript, expresiones regulares, fechas…) se escriben en el propio archivo.

## En el navegador

La zona de pruebas de la página de inicio ejecuta Solidify en tu navegador, compilado a WebAssembly: nada sale de tu máquina. El sitio también es una app: ábrelo en Chrome, Edge o Safari y elige **Instalar la app** (en un iPhone, *Compartir → Añadir a pantalla de inicio*). Después funciona sin conexión, zona de pruebas incluida.

## Como biblioteca Rust

Añádelo a tu `Cargo.toml`:

```toml
[dependencies]
solidify = { git = "https://github.com/maxlestage/Solidify" }
```

Después llámalo desde Rust:

```rust
let rust: String = solidify::transpile(typescript, &solidify::Options::default())?;
```

La página de la [biblioteca Rust](library.md) muestra toda la API.

## Desde el código fuente

```sh
git clone https://github.com/maxlestage/Solidify
cd Solidify
cargo build --release -p solidify
./target/release/solidify main.ts -o app.rs
```

El binario `solidify` acepta las mismas opciones que `npx solidify-rs`. Solidify no tiene dependencias: para compilarlo solo hace falta Rust.
