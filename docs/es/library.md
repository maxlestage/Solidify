# Biblioteca Rust

El transpilador es un crate de Rust, `solidify`, sin dependencias. La línea de comandos y la zona de pruebas del sitio web se basan en la misma API.

## Añadirlo

```toml
[dependencies]
solidify = { git = "https://github.com/maxlestage/Solidify" }
```

## Traducir un código fuente

`transpile` traduce un código fuente TypeScript o JavaScript, dado como cadena:

```rust
let typescript = "const n: number = 6 * 7;\nconsole.log(n);";
let rust: String = solidify::transpile(typescript, &solidify::Options::default())?;
```

Devuelve el Rust como un `String`, o un `solidify::Error`. El código fuente no puede importar otros archivos: usa `transpile_file` para un programa en disco.

## Traducir un programa

`transpile_file` lee el archivo de entrada en `path`, sigue sus imports (archivos relativos, paquetes de `node_modules`, módulos de Node) y traduce todo el programa a un único archivo Rust:

```rust
use std::path::Path;

let rust: String = solidify::transpile_file(Path::new("main.ts"), &solidify::Options::default())?;
std::fs::write("app.rs", rust)?;
```

Su error es un `String`, ya formateado con el archivo, la línea y la columna.

## Opciones

```rust
pub struct Options {
    /// The source file name, mentioned in the header of the output.
    pub file_name: Option<String>,
    /// Omit the header comment.
    pub no_header: bool,
}
```

`Options::default()` escribe un encabezado sin nombre de archivo. Para nombrar el código fuente, o para omitir el encabezado:

```rust
let options = solidify::Options {
    file_name: Some("main.ts".to_string()),
    no_header: false,
};
```

## Errores

Un `solidify::Error` tiene un `message` y un `span`, las posiciones en bytes del problema en el código fuente. Implementa `std::error::Error` y `Display` (solo el mensaje), y se formatea con el código fuente del que proviene:

- `error.position(source)`: la línea y la columna, contando desde 1;
- `error.display(source)`: `line:column: message`;
- `error.pretty(source)`: el mensaje, el lugar, la línea y una marca bajo ella, como los imprime la línea de comandos.

```rust
let source = std::fs::read_to_string("app.ts")?;
match solidify::transpile(&source, &solidify::Options::default()) {
    Ok(rust) => println!("{rust}"),
    Err(error) => eprintln!("{}", error.pretty(&source)),
}
```

## Proyecto

`Project` es lo que usan las funciones anteriores: los archivos de un programa, cargados y analizados. `Project::load(path)` lee un archivo de entrada y todo lo que importa (un `LoadError` nombra el archivo que ha fallado), `Project::single(name, source)` recibe un único código fuente, y `project.transpile(&options)` traduce. `project.pretty(&error)` formatea un error con el archivo en el que está, para los programas de varios archivos.

```rust
use solidify::{Options, Project};

let project = Project::load(std::path::Path::new("main.ts"))?;
match project.transpile(&Options::default()) {
    Ok(rust) => std::fs::write("app.rs", rust)?,
    Err(error) => eprintln!("{}", project.pretty(&error)),
}
```

## En el navegador

El crate se compila a WebAssembly tal cual: la zona de pruebas de la página de inicio llama a `transpile` en tu navegador, y el paquete npm es la línea de comandos compilada para WASI.
