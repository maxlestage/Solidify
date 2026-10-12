# Límites

El programa generado imprime lo que imprime Node.js; algunas cosas no pueden ser idénticas, y unas pocas se rechazan. Cada rechazo viene con un mensaje que dice qué y dónde.

## Qué sigue siendo distinto

- **Errores no capturados.** Un error impreso entero muestra `Error: message`, sin la pila de llamadas de JavaScript. Un error que nadie captura detiene el programa con el código de salida 1.
- **Los temporizadores** usan el reloj real: dos temporizadores separados por pocos milisegundos pueden intercambiarse con mucha carga, como puede ocurrir en Node.js.
- **El código construido en tiempo de ejecución** para `eval` o `new Function` se ejecuta en un pequeño intérprete escrito en el archivo, más lento que el código traducido.
- **Las fechas** se leen y se muestran en UTC.
- **Los números** que siguen siendo enteros son `i64`: un programa que cuenta con que los enteros por encima de 2⁵³ pierdan precisión, como ocurre con los dobles de JavaScript, obtiene en su lugar resultados exactos.

## Qué se rechaza

| Qué | Por qué |
| --- | --- |
| `async function*` | Los generadores asíncronos no se admiten. |
| `http`, `net`, `child_process` y los demás módulos de Node | Solo se incluyen `fs`, `fs/promises`, `path`, `os` y `process`. |
| Módulos nativos en `node_modules` | Son código compilado, no fuentes que traducir. |
| El DOM, `window`, `fetch` | Solidify traduce programas, no páginas para el navegador. |
| `Date.UTC` | Escribe la fecha como una cadena ISO. |
| Una variable que cambia de tipo | Usa dos variables, o declara la unión que quieres decir. |

## Modo estricto

Solidify lee TypeScript como lo hace su modo estricto: leer un valor que puede ser `null` o `undefined` es un error hasta que se comprueba, y una variable conserva el tipo de su primer valor salvo que su anotación diga otra cosa. [Errores](errors.md#los-mensajes-de-solidify) muestra los mensajes y qué cambiar.

## Informar de una diferencia

Un programa que se traduce pero imprime algo distinto de lo que imprime Node.js es un bug. Infórmalo en [GitHub](https://github.com/maxlestage/Solidify/issues) con el programa y las dos salidas: una vez corregido, se convierte en un programa de prueba.
