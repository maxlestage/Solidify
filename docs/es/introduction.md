# Introducción

Solidify lee un programa TypeScript o JavaScript, incluidos los archivos que importa, y escribe el Rust que habrías escrito a mano: structs, enums, `Option`, `Result`, iteradores y propiedad. La salida es un único archivo Rust normal, sin máquina virtual ni recolector de basura, e imprime lo que imprime Node.js.

## Qué hace

Dale este TypeScript:

```ts
interface Point { x: number; y: number; label?: string }

function describe(p: Point): string {
  if (p.label !== undefined) {
    return p.label;
  }
  return `(${p.x}, ${p.y})`;
}

const points: Point[] = [{ x: 1, y: 2 }, { x: 3, y: 4, label: "far" }];
for (const p of points) {
  console.log(describe(p));
}
```
```rust
#[derive(Debug, Clone, PartialEq)]
struct Point {
    pub x: i64,
    pub y: i64,
    pub label: Option<String>,
}

fn describe(p: &Point) -> String {
    if let Some(label) = &p.label {
        return label.clone();
    }
    format!("({}, {})", p.x, p.y)
}

fn main() {
    let points = vec![Point { x: 1, y: 2, label: None }, Point { x: 3, y: 4, label: Some("far".to_string()) }];
    for p in &points {
        println!("{}", describe(p));
    }
}
```

Cada decisión de esa salida viene del propio programa:

- `Point` es una interfaz, así que se convierte en un `struct`; el campo opcional `label` se convierte en un `Option<String>`.
- `x` e `y` solo contienen números enteros, así que son `i64`. Un literal decimal, una división o JSON los habrían convertido en `f64`.
- `describe` solo lee su argumento, así que lo toma prestado (`&Point`) en lugar de quedárselo.
- `p.label !== undefined` es una comprobación del `Option`: se convierte en `if let Some(label)`.
- Las instrucciones del nivel superior del archivo se convierten en `main`.

## Para qué sirve

- **Llevar un programa a Rust** sin reescribirlo desde cero: tradúcelo, luego lee el Rust y quédate con él.
- **Aprender Rust a partir de código que conoces:** escribe en TypeScript lo que quieres decir y mira cómo lo dice Rust.
- **Distribuir un solo binario** donde tenías un script de Node.js: `rustc` compila la salida, sin entorno de ejecución que instalar.

> [!NOTE]
> Solidify traduce programas, no bibliotecas para el navegador: no hay DOM, ni `fetch`, ni servidor `http`. Los [límites](limits.md) enumeran lo que sigue siendo distinto de Node.js y lo que se rechaza con un mensaje.

## Cómo se comprueba

El repositorio contiene 54 programas de prueba. Cada uno se traduce, se compila con `rustc` y se ejecuta; su salida debe ser exactamente lo que imprime Node.js con el mismo programa. Cada ejemplo de estas páginas se comprueba del mismo modo con el transpilador: lo que lees aquí es lo que escribe Solidify.

## Cómo leer estas páginas

- **Para empezar:** [instálalo](installation.md) y después sigue el [inicio rápido](quick-start.md), de un archivo `.ts` a un binario que se ejecuta.
- **La traducción:** [cómo funciona](how-it-works.md), y después una página por tema: [tipos](types.md), [funciones y propiedad](ownership.md), [clases e interfaces](classes.md), [errores](errors.md), [async y temporizadores](async.md), [módulos y programas](modules.md) y [la biblioteca JavaScript](runtime.md) (`console.log`, cadenas, arrays, JSON, expresiones regulares, fechas).
- **Referencia:** la [línea de comandos](cli.md), la [biblioteca Rust](library.md), la tabla completa de [qué se traduce en qué](mapping.md), los [límites](limits.md) y las [preguntas frecuentes](faq.md).

Pulsa `/` en cualquier parte del sitio para buscar en estas páginas.
