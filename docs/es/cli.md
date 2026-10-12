# Línea de comandos

`npx solidify-rs` ejecuta Solidify sobre tus archivos: el paquete npm contiene el transpilador compilado a WebAssembly, ejecutado por Node.js 20 o posterior. Compilado desde el código fuente, el binario `solidify` acepta las mismas opciones.

## Uso

```sh
npx solidify-rs [input.ts] [-o output.rs] [--check] [--no-header]
```

Traduce `input.ts` y todos los archivos que importa a un único archivo Rust. Sin archivo de entrada, lee la entrada estándar; sin `-o`, escribe en la salida estándar.

```sh
npx solidify-rs main.ts -o app.rs     # follows the imports of main.ts, one Rust file out
npx solidify-rs < app.ts > app.rs     # one file through the standard input
npx solidify-rs --check main.ts       # only report errors
```

## Opciones

| Opción | Qué hace |
| --- | --- |
| `input.ts` | El archivo de entrada del programa, TypeScript o JavaScript. `-` o nada lee la entrada estándar, donde no se siguen los imports. |
| `-o file`, `--output file` | Escribe el Rust en `file` en lugar de en la salida estándar. |
| `--check` | Traduce sin escribir nada: el código de salida y los mensajes dicen si el programa se traduce. |
| `--no-header` | Omite el comentario que nombra el archivo fuente al principio de la salida. |
| `-h`, `--help` | Imprime el modo de uso. |
| `-V`, `--version` | Imprime la versión. |

## Códigos de salida

| Código | Significado |
| --- | --- |
| `0` | El programa se ha traducido (o comprobado, con `--check`). |
| `1` | El programa no se traduce: el mensaje, el archivo, la línea y la columna están en la salida de error estándar. |
| `2` | El propio comando ha fallado: una opción desconocida, `-o` sin archivo, un archivo que no se puede leer o escribir. |

## Errores

Los mensajes van a la salida de error estándar, uno por problema encontrado, con la línea y una marca bajo el lugar:

```text
error: `name` may be null here: test it with `if (… !== null)` first
  --> app.ts:2:10
   |
  2|   return name.charAt(0);
   |          ^^^^
```

[Errores](errors.md#los-mensajes-de-solidify) explica los más comunes.

## En una compilación

`--check` sirve como prueba rápida para la integración continua, y los códigos de salida permiten que un script se detenga en el primer programa que no se traduce:

```sh
for f in scripts/*.ts; do
  npx solidify-rs --check "$f" || exit 1
done
```

Para mantener al día un programa traducido, tradúcelo y compílalo en el mismo paso:

```sh
npx solidify-rs main.ts -o src/main.rs && cargo build --release
```

## Compilar la salida

```sh
rustc --edition 2024 -O app.rs -o app
```

La salida no necesita ningún crate. Ponla en un proyecto Cargo como `src/main.rs` para usar las herramientas de Cargo (`cargo build`, `cargo clippy`); fija `edition = "2024"` en su `Cargo.toml`.
