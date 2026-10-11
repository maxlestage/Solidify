//! Interface texts in English, French and Spanish.

use crate::routes::Lang;

pub struct Dict {
    pub lang_name: &'static str,
    pub locale: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub nav: Nav,
    pub hero: Hero,
    pub play: Play,
    pub how: How,
    pub supported: Supported,
    pub install: Install,
    pub footer: Footer,
}

pub struct Nav {
    pub playground: &'static str,
    pub how: &'static str,
    pub supported: &'static str,
    pub install: &'static str,
    pub github: &'static str,
    pub skip: &'static str,
    /// The button that installs the site as an app, shown when the browser offers it.
    pub install_app: &'static str,
}

pub struct Hero {
    pub eyebrow: &'static str,
    pub title: &'static str,
    pub lead: &'static str,
    pub cta: &'static str,
    pub cta2: &'static str,
    pub facts: [(&'static str, &'static str); 3],
}

pub struct Play {
    pub title: &'static str,
    pub lead: &'static str,
    pub ts: &'static str,
    pub rust: &'static str,
    pub examples: &'static str,
    pub copy: &'static str,
    pub copied: &'static str,
    pub server: &'static str,
    pub browser: &'static str,
    pub lines: &'static str,
    pub error: &'static str,
}

pub struct How {
    pub title: &'static str,
    pub lead: &'static str,
    pub steps: [(&'static str, &'static str); 3],
    pub rules_title: &'static str,
    pub rules: [(&'static str, &'static str); 4],
}

pub struct Supported {
    pub title: &'static str,
    pub lead: &'static str,
    pub head: (&'static str, &'static str),
    pub rows: &'static [(&'static str, &'static str)],
    pub limits_title: &'static str,
    pub limits: &'static str,
}

pub struct Install {
    pub title: &'static str,
    pub lead: &'static str,
    pub cli: &'static str,
    pub library: &'static str,
    pub check: &'static str,
}

pub struct Footer {
    pub made: &'static str,
    pub family: &'static str,
    pub license: &'static str,
    pub source: &'static str,
}

pub fn t(lang: Lang) -> &'static Dict {
    match lang {
        Lang::En => &EN,
        Lang::Fr => &FR,
        Lang::Es => &ES,
    }
}

static EN: Dict = Dict {
    lang_name: "English",
    locale: "en_US",
    title: "Solidify, TypeScript to Rust",
    description: "Write TypeScript or JavaScript, get the Rust you would have written by hand: structs, enums, Option, Result, iterators and ownership. A transpiler in Rust, with no dependency, running here in your browser.",
    nav: Nav {
        playground: "Playground",
        how: "How it works",
        supported: "What translates",
        install: "Install",
        github: "GitHub",
        skip: "Skip to content",
        install_app: "Install the app",
    },
    hero: Hero {
        eyebrow: "A transpiler written in Rust, no dependency",
        title: "Write *TypeScript*. Ship *Rust*.",
        lead: "Solidify reads TypeScript or JavaScript and writes the Rust you would write by hand: structs, enums, Option, Result, iterators, ownership. No virtual machine and no garbage collector in the output, a plain Rust program.",
        cta: "Try it in the browser",
        cta2: "Install the CLI",
        facts: [
            ("0", "dependencies"),
            ("54", "programs checked against Node.js"),
            ("1", "Rust file out, however many files in"),
        ],
    },
    play: Play {
        title: "Playground",
        lead: "Edit the TypeScript on the left. The Rust on the right comes from Solidify itself, compiled to WebAssembly and running in your browser.",
        ts: "TypeScript",
        rust: "Rust",
        examples: "Examples",
        copy: "Copy",
        copied: "Copied",
        server: "translated on the server",
        browser: "translated in your browser",
        lines: "lines",
        error: "Solidify cannot translate this yet",
    },
    how: How {
        title: "How it works",
        lead: "Three passes, all in Rust, all deterministic. The same code runs in the command line and in this page.",
        steps: [
            (
                "Parse",
                "A hand-written parser reads modern TypeScript: classes, interfaces, enums, arrow functions, template strings, destructuring, async/await. JavaScript is TypeScript without the annotations.",
            ),
            (
                "Infer",
                "Types come from annotations and from use, by unification. Numbers stay integers until a float literal or a division says otherwise. `T | null` becomes `Option<T>`. A function that throws returns `Result`.",
            ),
            (
                "Generate",
                "Every value gets an ownership: borrowed, moved on its last use, cloned otherwise. Loops become ranges and iterators, `switch` becomes `match`, `if (x !== null)` becomes `if let Some(x)`.",
            ),
        ],
        rules_title: "Four rules worth knowing",
        rules: [
            (
                "Integers first",
                "`let n = 0` is an `i64`. Write `2.0`, divide, or read the number from JSON, and the whole family of that number becomes `f64`. `parseInt` gives `Option<i64>`: `NaN` is `None`.",
            ),
            (
                "Null is Option",
                "`T | null`, `T | undefined` and `x?: T` are `Option<T>`. Test them with `!== null` or `??`, and the Rust uses `if let`, `let … else` and `unwrap_or`.",
            ),
            (
                "Throw is Result",
                "A function that throws, or calls one that does, returns `Result<T, String>`. Calls get `?`, a `try` block becomes a closure, `catch (e)` receives the message.",
            ),
            (
                "Borrow by default",
                "Parameters are `&str`, `&[T]` and `&T` unless the function changes or keeps them. A variable is moved at its last use and cloned before that.",
            ),
        ],
    },
    supported: Supported {
        title: "What translates to what",
        lead: "The supported subset is the one most programs are made of. Anything else is refused with a message that says why and what to write instead.",
        head: ("TypeScript", "Rust"),
        rows: &[
            (
                "number",
                "i64, or f64 when a float is involved or the value can outgrow i64",
            ),
            ("bigint, 10n ** 30n", "JsBigInt, integers of any size"),
            (
                "string, template strings, tag`…`",
                "String, &str for parameters, format!, a call of the tag",
            ),
            ("T[], [A, B]", "Vec<T>, &[T] for parameters, tuples"),
            (
                "T | null, x?: T, ??, ?.",
                "Option<T>, if let, unwrap_or, map",
            ),
            (
                "interface, object literal",
                "struct with Debug, Clone, PartialEq",
            ),
            (
                "an object stored in two places, ===",
                "Shared<T> (Rc<RefCell<T>>): one object, compared by identity",
            ),
            (
                "class, get, set, static, static { … }",
                "struct, impl, new(), &self or &mut self, statics of the module",
            ),
            (
                "const A = class { … }, function F() { this.x = … }, F.prototype.m",
                "classes, rebuilt as structs and impl blocks",
            ),
            ("enum, \"a\" | \"b\"", "enum with Display, match"),
            (
                "Map, Set, Record<K, V>, { [k: string]: V }",
                "HashMap, HashSet, or JsMap, JsSet in insertion order",
            ),
            (
                "A & B, keyof T, T[\"k\"], typeof x, `id-${number}`",
                "one struct with the fields of both, string, the field's type, the variable's type",
            ),
            (
                "throw, try / catch / finally",
                "Result<T, String>, ?, a closure for the try block",
            ),
            (
                "class NotFound extends Error, instanceof in catch",
                "errors that keep their class, fields, message and name",
            ),
            (
                "map, filter, reduce, find, some, forEach…",
                "iterator chains",
            ),
            (
                "for … of, for (;;), while, labels, for await",
                "for x in &v, for i in a..b, loop, 'label, .await per item",
            ),
            (
                "switch, case without break",
                "match with grouped cases, labeled blocks for the fallthrough",
            ),
            (
                "async, await, new Promise",
                "fn … -> Promise<T>: a task started at once, .await",
            ),
            (
                "then, catch, finally, Promise.all, allSettled, race, any",
                "methods of Promise<T>, microtasks in Node's order",
            ),
            (
                "setTimeout, setInterval",
                "an event loop run by main, with Node's timings",
            ),
            (
                "function f<T>(x: T), f<number>(x)",
                "fn f<T: Clone + PartialEq>(x: T), bounds from use",
            ),
            (
                "class Stack<T>, interface Page<T>, <T extends HasId>",
                "struct Stack<T> + impl<T: …>, bounds found across the program",
            ),
            (
                "class B extends A, abstract, super, instanceof",
                "flattened structs, a trait per base class, Box<dyn ALike>",
            ),
            (
                "interface Shape { area(): number }, implements",
                "a trait per interface, impl for each class, Box<dyn ShapeLike>",
            ),
            (
                "A | B, discriminated unions, typeof, \"kind\" in x",
                "enum with one variant per member, match",
            ),
            (
                "any, unknown",
                "JsValue: a value known at run time, with JavaScript's operators",
            ),
            (
                "eval, new Function",
                "the code inlined when it is known, a small interpreter otherwise",
            ),
            (
                "Proxy, Reflect, Symbol, arguments",
                "JsProxy with its traps, JsSymbol, a rest parameter",
            ),
            ("function*, yield, yield*", "JsGenerator<T>, for x in gen"),
            (
                "/regex/flags, lookbehind, test, exec, match, replace, split",
                "JsRegex, a backtracking engine shipped in the file",
            ),
            (
                "JSON.stringify, JSON.parse, replacer, reviver",
                "the value's JsValue, JsFromJson for the parsed type",
            ),
            (
                "import, export, import * as ns, require, module.exports",
                "one mod per file, pub for exports",
            ),
            (
                "import fs from \"node:fs\", path, os, process, node_modules",
                "modules shipped with Solidify, packages translated from their sources",
            ),
            (
                "console.log of objects, arrays, maps, sets",
                "the exact text Node.js prints",
            ),
            (
                "...rest, f(...xs), { ...base, z }, { a, ...rest }, ({ a: { b } }) =>",
                "slices, vec!, struct literals field by field, nested patterns taken apart",
            ),
            (
                "new Date(…), getFullYear, toISOString, later - earlier",
                "JsDate in milliseconds (UTC), PartialOrd",
            ),
            (
                "f?.(), obj?.method(), fields of function type",
                "if let Some(f) = …, .map(|o| o.method()), Rc<dyn Fn(…)> fields",
            ),
            ("recursive classes", "Option<Box<T>>"),
        ],
        limits_title: "What stays different",
        limits: "The generated program prints what Node.js prints; a few things cannot be the same. An error printed whole shows `Error: message`, without JavaScript's stack trace. Timers use the real clock: two timers a few milliseconds apart may swap under heavy load, as they can in Node. Code built at run time for `eval` or `new Function` runs in a small interpreter shipped in the file, slower than translated code. Of Node's own modules, `fs`, `fs/promises`, `path`, `os` and `process` are there; packages from `node_modules` are translated from their sources, so native add-ons and modules such as `http` or `child_process` are refused, like async generators (`async function*`), with a message that says why.",
    },
    install: Install {
        title: "Install",
        lead: "Nothing to install but Node.js: npx runs Solidify, compiled to WebAssembly, on your files. Translate a file and the files it imports, or pipe one file through the standard input.",
        cli: "With npx",
        library: "As a library",
        check: "Check without writing",
    },
    footer: Footer {
        made: "Designed and developed by",
        family: "as part of one family of Rust projects:",
        license: "Proprietary software, all rights reserved. Free to use as a dependency of your programs.",
        source: "Source on GitHub",
    },
};

static FR: Dict = Dict {
    lang_name: "Français",
    locale: "fr_FR",
    title: "Solidify, de TypeScript à Rust",
    description: "Écrivez du TypeScript ou du JavaScript, obtenez le Rust que vous auriez écrit à la main : structs, enums, Option, Result, itérateurs et propriété. Un transpileur en Rust, sans dépendance, qui tourne ici dans votre navigateur.",
    nav: Nav {
        playground: "Bac à sable",
        how: "Fonctionnement",
        supported: "Ce qui se traduit",
        install: "Installer",
        github: "GitHub",
        skip: "Aller au contenu",
        install_app: "Installer l'app",
    },
    hero: Hero {
        eyebrow: "Un transpileur écrit en Rust, sans dépendance",
        title: "Écrivez du *TypeScript*. Livrez du *Rust*.",
        lead: "Solidify lit du TypeScript ou du JavaScript et écrit le Rust que vous auriez écrit à la main : structs, enums, Option, Result, itérateurs, propriété. Ni machine virtuelle ni ramasse-miettes en sortie, un programme Rust ordinaire.",
        cta: "Essayer dans le navigateur",
        cta2: "Installer la ligne de commande",
        facts: [
            ("0", "dépendance"),
            ("54", "programmes vérifiés contre Node.js"),
            (
                "1",
                "fichier Rust en sortie, autant de fichiers en entrée que voulu",
            ),
        ],
    },
    play: Play {
        title: "Bac à sable",
        lead: "Modifiez le TypeScript à gauche. Le Rust à droite vient de Solidify lui-même, compilé en WebAssembly et exécuté dans votre navigateur.",
        ts: "TypeScript",
        rust: "Rust",
        examples: "Exemples",
        copy: "Copier",
        copied: "Copié",
        server: "traduit sur le serveur",
        browser: "traduit dans votre navigateur",
        lines: "lignes",
        error: "Solidify ne sait pas encore traduire ceci",
    },
    how: How {
        title: "Fonctionnement",
        lead: "Trois passes, toutes en Rust, toutes déterministes. Le même code tourne en ligne de commande et dans cette page.",
        steps: [
            (
                "Analyser",
                "Un parseur écrit à la main lit le TypeScript moderne : classes, interfaces, enums, fonctions fléchées, gabarits de chaînes, déstructuration, async/await. Le JavaScript est du TypeScript sans annotations.",
            ),
            (
                "Inférer",
                "Les types viennent des annotations et de l'usage, par unification. Les nombres restent des entiers tant qu'un littéral flottant ou une division ne dit pas le contraire. `T | null` devient `Option<T>`. Une fonction qui lève une erreur renvoie `Result`.",
            ),
            (
                "Générer",
                "Chaque valeur reçoit une propriété : empruntée, déplacée à son dernier usage, clonée sinon. Les boucles deviennent des intervalles et des itérateurs, `switch` devient `match`, `if (x !== null)` devient `if let Some(x)`.",
            ),
        ],
        rules_title: "Quatre règles à connaître",
        rules: [
            (
                "Les entiers d'abord",
                "`let n = 0` est un `i64`. Écrivez `2.0`, divisez, ou lisez le nombre depuis du JSON, et toute la famille de ce nombre devient `f64`. `parseInt` donne un `Option<i64>` : `NaN` est `None`.",
            ),
            (
                "Null est Option",
                "`T | null`, `T | undefined` et `x?: T` sont des `Option<T>`. Testez-les avec `!== null` ou `??`, et le Rust utilise `if let`, `let … else` et `unwrap_or`.",
            ),
            (
                "Throw est Result",
                "Une fonction qui lève une erreur, ou qui en appelle une qui le fait, renvoie `Result<T, String>`. Les appels reçoivent `?`, un bloc `try` devient une closure, `catch (e)` reçoit le message.",
            ),
            (
                "Emprunter par défaut",
                "Les paramètres sont `&str`, `&[T]` et `&T` sauf si la fonction les modifie ou les conserve. Une variable est déplacée à son dernier usage et clonée avant.",
            ),
        ],
    },
    supported: Supported {
        title: "Ce qui se traduit, et en quoi",
        lead: "Le sous-ensemble pris en charge est celui dont la plupart des programmes sont faits. Le reste est refusé avec un message qui dit pourquoi, et quoi écrire à la place.",
        head: ("TypeScript", "Rust"),
        rows: &[
            (
                "number",
                "i64, ou f64 dès qu'un flottant intervient ou que la valeur peut dépasser i64",
            ),
            ("bigint, 10n ** 30n", "JsBigInt, entiers de toute taille"),
            (
                "string, template strings, tag`…`",
                "String, &str en paramètre, format!, un appel du tag",
            ),
            ("T[], [A, B]", "Vec<T>, &[T] en paramètre, tuples"),
            (
                "T | null, x?: T, ??, ?.",
                "Option<T>, if let, unwrap_or, map",
            ),
            (
                "interface, object literal",
                "struct avec Debug, Clone, PartialEq",
            ),
            (
                "an object stored in two places, ===",
                "Shared<T> (Rc<RefCell<T>>) : un seul objet, comparé par identité",
            ),
            (
                "class, get, set, static, static { … }",
                "struct, impl, new(), &self ou &mut self, statics du module",
            ),
            (
                "const A = class { … }, function F() { this.x = … }, F.prototype.m",
                "des classes, reconstruites en structs et blocs impl",
            ),
            ("enum, \"a\" | \"b\"", "enum avec Display, match"),
            (
                "Map, Set, Record<K, V>, { [k: string]: V }",
                "HashMap, HashSet, ou JsMap, JsSet dans l'ordre d'insertion",
            ),
            (
                "A & B, keyof T, T[\"k\"], typeof x, `id-${number}`",
                "un struct avec les champs des deux, string, le type du champ, celui de la variable",
            ),
            (
                "throw, try / catch / finally",
                "Result<T, String>, ?, une closure pour le bloc try",
            ),
            (
                "class NotFound extends Error, instanceof in catch",
                "des erreurs qui gardent leur classe, leurs champs, leur message et leur nom",
            ),
            (
                "map, filter, reduce, find, some, forEach…",
                "chaînes d'itérateurs",
            ),
            (
                "for … of, for (;;), while, labels, for await",
                "for x in &v, for i in a..b, loop, 'label, .await par élément",
            ),
            (
                "switch, case without break",
                "match avec cas groupés, blocs étiquetés pour les cas qui s'enchaînent",
            ),
            (
                "async, await, new Promise",
                "fn … -> Promise<T> : une tâche lancée tout de suite, .await",
            ),
            (
                "then, catch, finally, Promise.all, allSettled, race, any",
                "méthodes de Promise<T>, microtâches dans l'ordre de Node",
            ),
            (
                "setTimeout, setInterval",
                "une boucle d'événements lancée par main, aux délais de Node",
            ),
            (
                "function f<T>(x: T), f<number>(x)",
                "fn f<T: Clone + PartialEq>(x: T), bornes déduites de l'usage",
            ),
            (
                "class Stack<T>, interface Page<T>, <T extends HasId>",
                "struct Stack<T> + impl<T: …>, bornes trouvées dans tout le programme",
            ),
            (
                "class B extends A, abstract, super, instanceof",
                "structs aplatis, un trait par classe de base, Box<dyn ALike>",
            ),
            (
                "interface Shape { area(): number }, implements",
                "un trait par interface, impl pour chaque classe, Box<dyn ShapeLike>",
            ),
            (
                "A | B, discriminated unions, typeof, \"kind\" in x",
                "enum avec une variante par membre, match",
            ),
            (
                "any, unknown",
                "JsValue : une valeur connue à l'exécution, avec les opérateurs de JavaScript",
            ),
            (
                "eval, new Function",
                "le code intégré quand il est connu, un petit interpréteur sinon",
            ),
            (
                "Proxy, Reflect, Symbol, arguments",
                "JsProxy et ses pièges, JsSymbol, un paramètre rest",
            ),
            ("function*, yield, yield*", "JsGenerator<T>, for x in gen"),
            (
                "/regex/flags, lookbehind, test, exec, match, replace, split",
                "JsRegex, un moteur à retour arrière livré dans le fichier",
            ),
            (
                "JSON.stringify, JSON.parse, replacer, reviver",
                "le JsValue de la valeur, JsFromJson pour le type lu",
            ),
            (
                "import, export, import * as ns, require, module.exports",
                "un mod par fichier, pub pour les exports",
            ),
            (
                "import fs from \"node:fs\", path, os, process, node_modules",
                "modules fournis avec Solidify, paquets traduits depuis leurs sources",
            ),
            (
                "console.log of objects, arrays, maps, sets",
                "le texte exact qu'affiche Node.js",
            ),
            (
                "...rest, f(...xs), { ...base, z }, { a, ...rest }, ({ a: { b } }) =>",
                "slices, vec!, littéraux de struct champ par champ, motifs imbriqués décomposés",
            ),
            (
                "new Date(…), getFullYear, toISOString, later - earlier",
                "JsDate en millisecondes (UTC), PartialOrd",
            ),
            (
                "f?.(), obj?.method(), fields of function type",
                "if let Some(f) = …, .map(|o| o.method()), champs Rc<dyn Fn(…)>",
            ),
            ("recursive classes", "Option<Box<T>>"),
        ],
        limits_title: "Ce qui reste différent",
        limits: "Le programme généré affiche ce qu'affiche Node.js ; quelques points ne peuvent pas être identiques. Une erreur affichée en entier donne `Error: message`, sans la pile d'appels de JavaScript. Les minuteries suivent l'horloge réelle : deux minuteries à quelques millisecondes d'écart peuvent s'inverser sous forte charge, comme dans Node. Le code construit à l'exécution pour `eval` ou `new Function` tourne dans un petit interpréteur livré dans le fichier, plus lent que le code traduit. Parmi les modules propres à Node, `fs`, `fs/promises`, `path`, `os` et `process` sont là ; les paquets de `node_modules` sont traduits depuis leurs sources, donc les modules natifs et des modules comme `http` ou `child_process` sont refusés, comme les générateurs asynchrones (`async function*`), avec un message qui dit pourquoi.",
    },
    install: Install {
        title: "Installer",
        lead: "Rien à installer d'autre que Node.js : npx lance Solidify, compilé en WebAssembly, sur vos fichiers. Traduisez un fichier et ceux qu'il importe, ou passez un fichier par l'entrée standard.",
        cli: "Avec npx",
        library: "Comme bibliothèque",
        check: "Vérifier sans écrire",
    },
    footer: Footer {
        made: "Conçu et développé par",
        family: "au sein d'une même famille de projets Rust :",
        license: "Logiciel propriétaire, tous droits réservés. Libre d'utilisation comme dépendance de vos programmes.",
        source: "Source sur GitHub",
    },
};

static ES: Dict = Dict {
    lang_name: "Español",
    locale: "es_ES",
    title: "Solidify, de TypeScript a Rust",
    description: "Escribe TypeScript o JavaScript y obtén el Rust que habrías escrito a mano: structs, enums, Option, Result, iteradores y propiedad. Un transpilador en Rust, sin dependencias, que se ejecuta aquí en tu navegador.",
    nav: Nav {
        playground: "Zona de pruebas",
        how: "Cómo funciona",
        supported: "Qué se traduce",
        install: "Instalar",
        github: "GitHub",
        skip: "Ir al contenido",
        install_app: "Instalar la app",
    },
    hero: Hero {
        eyebrow: "Un transpilador escrito en Rust, sin dependencias",
        title: "Escribe *TypeScript*. Entrega *Rust*.",
        lead: "Solidify lee TypeScript o JavaScript y escribe el Rust que habrías escrito a mano: structs, enums, Option, Result, iteradores, propiedad. Sin máquina virtual ni recolector de basura en la salida, un programa Rust normal.",
        cta: "Probar en el navegador",
        cta2: "Instalar la línea de comandos",
        facts: [
            ("0", "dependencias"),
            ("54", "programas verificados contra Node.js"),
            (
                "1",
                "archivo Rust de salida, tantos archivos de entrada como quieras",
            ),
        ],
    },
    play: Play {
        title: "Zona de pruebas",
        lead: "Edita el TypeScript de la izquierda. El Rust de la derecha lo produce el propio Solidify, compilado a WebAssembly y ejecutado en tu navegador.",
        ts: "TypeScript",
        rust: "Rust",
        examples: "Ejemplos",
        copy: "Copiar",
        copied: "Copiado",
        server: "traducido en el servidor",
        browser: "traducido en tu navegador",
        lines: "líneas",
        error: "Solidify todavía no sabe traducir esto",
    },
    how: How {
        title: "Cómo funciona",
        lead: "Tres pasadas, todas en Rust, todas deterministas. El mismo código se ejecuta en la línea de comandos y en esta página.",
        steps: [
            (
                "Analizar",
                "Un analizador escrito a mano lee TypeScript moderno: clases, interfaces, enums, funciones flecha, plantillas de cadenas, desestructuración, async/await. JavaScript es TypeScript sin anotaciones.",
            ),
            (
                "Inferir",
                "Los tipos salen de las anotaciones y del uso, por unificación. Los números siguen siendo enteros mientras un literal decimal o una división no digan lo contrario. `T | null` se convierte en `Option<T>`. Una función que lanza un error devuelve `Result`.",
            ),
            (
                "Generar",
                "Cada valor recibe una propiedad: prestado, movido en su último uso, clonado si no. Los bucles se vuelven rangos e iteradores, `switch` se vuelve `match`, `if (x !== null)` se vuelve `if let Some(x)`.",
            ),
        ],
        rules_title: "Cuatro reglas que conviene conocer",
        rules: [
            (
                "Primero los enteros",
                "`let n = 0` es un `i64`. Escribe `2.0`, divide o lee el número desde JSON, y toda la familia de ese número pasa a `f64`. `parseInt` da un `Option<i64>`: `NaN` es `None`.",
            ),
            (
                "Null es Option",
                "`T | null`, `T | undefined` y `x?: T` son `Option<T>`. Compruébalos con `!== null` o `??`, y el Rust usa `if let`, `let … else` y `unwrap_or`.",
            ),
            (
                "Throw es Result",
                "Una función que lanza un error, o que llama a otra que lo hace, devuelve `Result<T, String>`. Las llamadas reciben `?`, un bloque `try` se vuelve una closure, `catch (e)` recibe el mensaje.",
            ),
            (
                "Prestar por defecto",
                "Los parámetros son `&str`, `&[T]` y `&T` salvo que la función los modifique o los guarde. Una variable se mueve en su último uso y se clona antes.",
            ),
        ],
    },
    supported: Supported {
        title: "Qué se traduce, y a qué",
        lead: "El subconjunto admitido es aquel del que están hechos la mayoría de los programas. Lo demás se rechaza con un mensaje que explica por qué y qué escribir en su lugar.",
        head: ("TypeScript", "Rust"),
        rows: &[
            (
                "number",
                "i64, o f64 en cuanto interviene un decimal o el valor puede superar i64",
            ),
            (
                "bigint, 10n ** 30n",
                "JsBigInt, enteros de cualquier tamaño",
            ),
            (
                "string, template strings, tag`…`",
                "String, &str como parámetro, format!, una llamada a la etiqueta",
            ),
            ("T[], [A, B]", "Vec<T>, &[T] como parámetro, tuplas"),
            (
                "T | null, x?: T, ??, ?.",
                "Option<T>, if let, unwrap_or, map",
            ),
            (
                "interface, object literal",
                "struct con Debug, Clone, PartialEq",
            ),
            (
                "an object stored in two places, ===",
                "Shared<T> (Rc<RefCell<T>>): un solo objeto, comparado por identidad",
            ),
            (
                "class, get, set, static, static { … }",
                "struct, impl, new(), &self o &mut self, statics del módulo",
            ),
            (
                "const A = class { … }, function F() { this.x = … }, F.prototype.m",
                "clases, reconstruidas como structs y bloques impl",
            ),
            ("enum, \"a\" | \"b\"", "enum con Display, match"),
            (
                "Map, Set, Record<K, V>, { [k: string]: V }",
                "HashMap, HashSet, o JsMap, JsSet en orden de inserción",
            ),
            (
                "A & B, keyof T, T[\"k\"], typeof x, `id-${number}`",
                "un struct con los campos de ambos, string, el tipo del campo, el de la variable",
            ),
            (
                "throw, try / catch / finally",
                "Result<T, String>, ?, un closure para el bloque try",
            ),
            (
                "class NotFound extends Error, instanceof in catch",
                "errores que conservan su clase, campos, mensaje y nombre",
            ),
            (
                "map, filter, reduce, find, some, forEach…",
                "cadenas de iteradores",
            ),
            (
                "for … of, for (;;), while, labels, for await",
                "for x in &v, for i in a..b, loop, 'label, .await por elemento",
            ),
            (
                "switch, case without break",
                "match con casos agrupados, bloques etiquetados para los casos encadenados",
            ),
            (
                "async, await, new Promise",
                "fn … -> Promise<T>: una tarea lanzada al instante, .await",
            ),
            (
                "then, catch, finally, Promise.all, allSettled, race, any",
                "métodos de Promise<T>, microtareas en el orden de Node",
            ),
            (
                "setTimeout, setInterval",
                "un bucle de eventos lanzado por main, con los tiempos de Node",
            ),
            (
                "function f<T>(x: T), f<number>(x)",
                "fn f<T: Clone + PartialEq>(x: T), límites deducidos del uso",
            ),
            (
                "class Stack<T>, interface Page<T>, <T extends HasId>",
                "struct Stack<T> + impl<T: …>, límites hallados en todo el programa",
            ),
            (
                "class B extends A, abstract, super, instanceof",
                "structs aplanados, un trait por clase base, Box<dyn ALike>",
            ),
            (
                "interface Shape { area(): number }, implements",
                "un trait por interfaz, impl para cada clase, Box<dyn ShapeLike>",
            ),
            (
                "A | B, discriminated unions, typeof, \"kind\" in x",
                "enum con una variante por miembro, match",
            ),
            (
                "any, unknown",
                "JsValue: un valor conocido en ejecución, con los operadores de JavaScript",
            ),
            (
                "eval, new Function",
                "el código integrado cuando se conoce, un pequeño intérprete si no",
            ),
            (
                "Proxy, Reflect, Symbol, arguments",
                "JsProxy con sus trampas, JsSymbol, un parámetro rest",
            ),
            ("function*, yield, yield*", "JsGenerator<T>, for x in gen"),
            (
                "/regex/flags, lookbehind, test, exec, match, replace, split",
                "JsRegex, un motor con retroceso incluido en el archivo",
            ),
            (
                "JSON.stringify, JSON.parse, replacer, reviver",
                "el JsValue del valor, JsFromJson para el tipo leído",
            ),
            (
                "import, export, import * as ns, require, module.exports",
                "un mod por archivo, pub para los exports",
            ),
            (
                "import fs from \"node:fs\", path, os, process, node_modules",
                "módulos incluidos en Solidify, paquetes traducidos desde sus fuentes",
            ),
            (
                "console.log of objects, arrays, maps, sets",
                "el texto exacto que imprime Node.js",
            ),
            (
                "...rest, f(...xs), { ...base, z }, { a, ...rest }, ({ a: { b } }) =>",
                "slices, vec!, literales de struct campo a campo, patrones anidados descompuestos",
            ),
            (
                "new Date(…), getFullYear, toISOString, later - earlier",
                "JsDate en milisegundos (UTC), PartialOrd",
            ),
            (
                "f?.(), obj?.method(), fields of function type",
                "if let Some(f) = …, .map(|o| o.method()), campos Rc<dyn Fn(…)>",
            ),
            ("recursive classes", "Option<Box<T>>"),
        ],
        limits_title: "Qué sigue siendo distinto",
        limits: "El programa generado imprime lo que imprime Node.js; algunas cosas no pueden ser idénticas. Un error impreso entero muestra `Error: message`, sin la pila de llamadas de JavaScript. Los temporizadores siguen el reloj real: dos temporizadores separados por pocos milisegundos pueden intercambiarse con mucha carga, como en Node. El código construido en ejecución para `eval` o `new Function` corre en un pequeño intérprete incluido en el archivo, más lento que el código traducido. De los módulos propios de Node están `fs`, `fs/promises`, `path`, `os` y `process`; los paquetes de `node_modules` se traducen desde sus fuentes, así que los módulos nativos y módulos como `http` o `child_process` se rechazan, como los generadores asíncronos (`async function*`), con un mensaje que explica por qué.",
    },
    install: Install {
        title: "Instalar",
        lead: "Nada que instalar salvo Node.js: npx ejecuta Solidify, compilado a WebAssembly, sobre tus archivos. Traduce un archivo y los que importa, o pasa un archivo por la entrada estándar.",
        cli: "Con npx",
        library: "Como biblioteca",
        check: "Comprobar sin escribir",
    },
    footer: Footer {
        made: "Diseñado y desarrollado por",
        family: "dentro de una misma familia de proyectos Rust:",
        license: "Software propietario, todos los derechos reservados. Libre de usar como dependencia de tus programas.",
        source: "Código en GitHub",
    },
};
