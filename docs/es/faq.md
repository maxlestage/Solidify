# Preguntas frecuentes

Respuestas breves a las preguntas que se hacen sobre Solidify.

## ¿La salida es Rust legible?

Sí: está pensada para leerla y conservarla. Los structs, los enums, `Option`, `Result`, los parámetros prestados y las cadenas de iteradores son lo que escribirías a mano; los nombres siguen las convenciones de Rust y los comentarios se conservan. Las funciones auxiliares escritas después de `main` también son funciones Rust normales.

## ¿La salida necesita un entorno de ejecución o un crate?

No. La salida es un único archivo que `rustc` compila por sí solo, para la edición 2024. Lo que le falta a la biblioteca estándar de Rust (imprimir números como lo hace JavaScript, expresiones regulares, fechas, promesas y temporizadores) se escribe en el archivo, y solo lo que usa el programa.

## ¿El programa traducido es más rápido?

Normalmente sí: se compila por adelantado, sin recolector de basura, y los números que siguen siendo enteros son enteros de máquina. El código construido en tiempo de ejecución con `eval` se ejecuta en un intérprete, y sigue siendo lento.

## ¿Puedo editar la salida?

Sí. Es tu programa, en Rust: tradúcelo una vez y después trabaja sobre el Rust. O conserva el TypeScript como fuente y tradúcelo en cada compilación (consulta la [línea de comandos](cli.md#en-una-compilación)).

## ¿Qué TypeScript lee?

TypeScript y JavaScript modernos: clases, interfaces, genéricos, uniones, enums, `async`/`await`, generadores, desestructuración, encadenamiento opcional, módulos (ES y CommonJS). No ejecuta el comprobador de tipos de TypeScript: infiere sus propios tipos, que son más estrictos con `null`.

## ¿Por qué un número es un i64 y no un f64?

Porque la mayoría de los números de los programas son recuentos e índices. Un número sigue siendo entero hasta que el programa le da un valor decimal, lo divide o lo lee de JSON; entonces es un `f64`, y también lo es cada número al que fluye. La salida imprime ambos como lo haría JavaScript.

## ¿Funciona en el navegador?

El transpilador sí: la zona de pruebas de la página de inicio lo ejecuta, compilado a WebAssembly, y el sitio funciona sin conexión una vez instalado como app. Los programas que traduce son programas de línea de comandos: no tienen DOM.

## ¿Envía mi código a algún sitio?

No. `npx solidify-rs` se ejecuta en tu máquina, y la zona de pruebas se ejecuta en tu navegador.

## ¿Puedo usarlo en mi proyecto, o en mi empresa?

Sí: puedes usar Solidify libremente, proyectos comerciales incluidos, y el Rust que escribe es tuyo. Solidify en sí es software propietario: modificarlo o redistribuir su código fuente requiere el permiso por escrito del autor. La [licencia](../../LICENSE) establece las condiciones.

## ¿Cómo informo de un bug?

Abre una incidencia en [GitHub](https://github.com/maxlestage/Solidify/issues) con el programa, lo que imprime Node.js y lo que imprime el Rust (o el mensaje de Solidify). Los programas que fallaron alguna vez se convierten en programas de prueba.
