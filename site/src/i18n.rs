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
    },
    hero: Hero {
        eyebrow: "A transpiler written in Rust, no dependency",
        title: "Write *TypeScript*. Ship *Rust*.",
        lead: "Solidify reads TypeScript or JavaScript and writes the Rust you would write by hand: structs, enums, Option, Result, iterators, ownership. No virtual machine and no garbage collector in the output, a plain Rust program.",
        cta: "Try it in the browser",
        cta2: "Install the CLI",
        facts: [
            ("0", "dependencies"),
            ("33", "programs checked against Node.js"),
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
            ("number", "i64, or f64 when a float is involved"),
            (
                "string, template strings",
                "String, &str for parameters, format!",
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
                "class, get, set, static",
                "struct, impl, new(), &self or &mut self, consts",
            ),
            ("enum, \"a\" | \"b\"", "enum with Display, match"),
            ("Map, Set, Record<K, V>", "HashMap, HashSet"),
            (
                "throw, try / catch / finally",
                "Result<T, String>, ?, a closure for the try block",
            ),
            ("map, filter, reduce, find, some…", "iterator chains"),
            (
                "for … of, for (;;), while, labels",
                "for x in &v, for i in a..b, loop, 'label",
            ),
            ("switch", "match, with grouped cases"),
            (
                "async, await",
                "async fn, .await, a small executor for main",
            ),
            (
                "function f<T>(x: T)",
                "fn f<T: Clone + PartialEq>(x: T), bounds from use",
            ),
            (
                "console.log, Math, parseInt, …",
                "println!, f64 methods, small helpers",
            ),
            ("recursive classes", "Option<Box<T>>"),
            (
                "class B extends A, abstract, super, instanceof",
                "flattened structs, a trait per base class, Box<dyn ALike> where an A is expected",
            ),
            (
                "interface Shape { area(): number }, class C implements Shape",
                "a trait per interface, impl for each class, Box<dyn ShapeLike>",
            ),
            (
                "A | B, [1, \"two\"], discriminated unions, typeof",
                "enum with one variant per member, match",
            ),
            ("function*, yield, yield*", "JsGenerator<T>, for x in gen"),
            (
                "setTimeout, setInterval, new Promise",
                "an event loop run by main, JsPromise",
            ),
            (
                "/regex/, test, exec, match, replace, split, lastIndex",
                "JsRegex, a backtracking engine shipped in the file, while let Some(m) = re.exec(&s)",
            ),
            (
                "JSON.stringify, JSON.parse",
                "the value's JsValue, JsFromJson for the parsed type",
            ),
            (
                "import, export, import * as ns",
                "one mod per file, pub for exports",
            ),
            (
                "console.log of objects, arrays, maps",
                "the exact text Node.js prints",
            ),
            ("any, unknown", "inferred from use"),
            (
                "...rest, f(...xs), { ...base, z }, [a, ...rest], ({ x, y }) =>",
                "slices, vec!, struct literals field by field, xs[1..].to_vec()",
            ),
            (
                "new Date(…), getFullYear, toISOString, later - earlier",
                "JsDate in milliseconds (UTC), PartialOrd, get_time() differences",
            ),
            (
                "f?.(), obj?.method(), fields of function type",
                "if let Some(f) = …, .map(|o| o.method()), Rc<dyn Fn(…)> fields",
            ),
            (
                "class Stack<T>, interface Page<T>, <T extends HasId>, f<number>(x)",
                "struct Stack<T> + impl<T: …>, bounds found across the program, constraint traits",
            ),
            (
                "interface Listener<T> { notify(v: T): void }",
                "trait ListenerLike<T>, Box<dyn ListenerLike<String>>",
            ),
        ],
        limits_title: "What stays different",
        limits: "Rust values are not JavaScript references: an object stored in two places is copied, and changing one copy leaves the other alone (variables changed inside closures that outlive them are shared). Numbers are `i64` until a float is involved, so an integer that overflows panics where JavaScript would lose precision. Imports go between your own files only, not to packages. `JSON` replacers and revivers, `Symbol`, `Proxy`, `eval`, prototypes and `arguments` have no Rust counterpart and are refused with a clear message.",
    },
    install: Install {
        title: "Install",
        lead: "One crate, no dependency, a single binary. Translate a file and the files it imports, or pipe one file through the standard input.",
        cli: "Command line",
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
    },
    hero: Hero {
        eyebrow: "Un transpileur écrit en Rust, sans dépendance",
        title: "Écrivez du *TypeScript*. Livrez du *Rust*.",
        lead: "Solidify lit du TypeScript ou du JavaScript et écrit le Rust que vous auriez écrit à la main : structs, enums, Option, Result, itérateurs, propriété. Ni machine virtuelle ni ramasse-miettes en sortie, un programme Rust ordinaire.",
        cta: "Essayer dans le navigateur",
        cta2: "Installer la ligne de commande",
        facts: [
            ("0", "dépendance"),
            ("33", "programmes vérifiés contre Node.js"),
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
            ("number", "i64, ou f64 dès qu'un flottant intervient"),
            (
                "string, gabarits de chaînes",
                "String, &str pour les paramètres, format!",
            ),
            ("T[], [A, B]", "Vec<T>, &[T] pour les paramètres, tuples"),
            (
                "T | null, x?: T, ??, ?.",
                "Option<T>, if let, unwrap_or, map",
            ),
            (
                "interface, objet littéral",
                "struct avec Debug, Clone, PartialEq",
            ),
            (
                "class, get, set, static",
                "struct, impl, new(), &self ou &mut self, consts",
            ),
            ("enum, \"a\" | \"b\"", "enum avec Display, match"),
            ("Map, Set, Record<K, V>", "HashMap, HashSet"),
            (
                "throw, try / catch / finally",
                "Result<T, String>, ?, une closure pour le bloc try",
            ),
            ("map, filter, reduce, find, some…", "chaînes d'itérateurs"),
            (
                "for … of, for (;;), while, étiquettes",
                "for x in &v, for i in a..b, loop, 'label",
            ),
            ("switch", "match, avec regroupement des cas"),
            (
                "async, await",
                "async fn, .await, un petit exécuteur pour main",
            ),
            (
                "function f<T>(x: T)",
                "fn f<T: Clone + PartialEq>(x: T), bornes déduites de l'usage",
            ),
            (
                "console.log, Math, parseInt, …",
                "println!, méthodes de f64, petits utilitaires",
            ),
            ("classes récursives", "Option<Box<T>>"),
            (
                "class B extends A, abstract, super, instanceof",
                "structs aplaties, un trait par classe de base, Box<dyn ALike> là où un A est attendu",
            ),
            (
                "interface Shape { area(): number }, class C implements Shape",
                "un trait par interface, impl pour chaque classe, Box<dyn ShapeLike>",
            ),
            (
                "A | B, [1, \"two\"], unions discriminées, typeof",
                "enum avec une variante par membre, match",
            ),
            ("function*, yield, yield*", "JsGenerator<T>, for x in gen"),
            (
                "setTimeout, setInterval, new Promise",
                "une boucle d'événements lancée par main, JsPromise",
            ),
            (
                "/regex/, test, exec, match, replace, split, lastIndex",
                "JsRegex, un moteur à retour arrière livré dans le fichier, while let Some(m) = re.exec(&s)",
            ),
            (
                "JSON.stringify, JSON.parse",
                "la JsValue de la valeur, JsFromJson pour le type lu",
            ),
            (
                "import, export, import * as ns",
                "un mod par fichier, pub pour les exports",
            ),
            (
                "console.log d'objets, tableaux, maps",
                "le texte exact qu'imprime Node.js",
            ),
            ("any, unknown", "déduits de l'usage"),
            (
                "...rest, f(...xs), { ...base, z }, [a, ...rest], ({ x, y }) =>",
                "slices, vec!, littéraux de struct champ par champ, xs[1..].to_vec()",
            ),
            (
                "new Date(…), getFullYear, toISOString, plus_tard - plus_tôt",
                "JsDate en millisecondes (UTC), PartialOrd, différences de get_time()",
            ),
            (
                "f?.(), obj?.method(), champs de type fonction",
                "if let Some(f) = …, .map(|o| o.method()), champs Rc<dyn Fn(…)>",
            ),
            (
                "class Stack<T>, interface Page<T>, <T extends HasId>, f<number>(x)",
                "struct Stack<T> + impl<T: …>, bornes trouvées dans tout le programme, traits de contrainte",
            ),
            (
                "interface Listener<T> { notify(v: T): void }",
                "trait ListenerLike<T>, Box<dyn ListenerLike<String>>",
            ),
        ],
        limits_title: "Ce qui reste différent",
        limits: "Les valeurs Rust ne sont pas des références JavaScript : un objet rangé à deux endroits est copié, et modifier une copie ne touche pas l'autre (les variables modifiées dans des closures qui leur survivent sont partagées). Les nombres sont des `i64` tant qu'un flottant n'intervient pas : un entier qui déborde fait paniquer là où JavaScript perdrait de la précision. Les imports vont entre vos propres fichiers, pas vers des paquets. Les replacers et revivers de `JSON`, `Symbol`, `Proxy`, `eval`, les prototypes et `arguments` n'ont pas d'équivalent Rust et sont refusés avec un message clair.",
    },
    install: Install {
        title: "Installer",
        lead: "Une crate, aucune dépendance, un seul binaire. Traduisez un fichier et ceux qu'il importe, ou passez un fichier par l'entrée standard.",
        cli: "Ligne de commande",
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
    },
    hero: Hero {
        eyebrow: "Un transpilador escrito en Rust, sin dependencias",
        title: "Escribe *TypeScript*. Entrega *Rust*.",
        lead: "Solidify lee TypeScript o JavaScript y escribe el Rust que habrías escrito a mano: structs, enums, Option, Result, iteradores, propiedad. Sin máquina virtual ni recolector de basura en la salida, un programa Rust normal.",
        cta: "Probar en el navegador",
        cta2: "Instalar la línea de comandos",
        facts: [
            ("0", "dependencias"),
            ("33", "programas verificados contra Node.js"),
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
            ("number", "i64, o f64 en cuanto interviene un decimal"),
            (
                "string, plantillas de cadenas",
                "String, &str para los parámetros, format!",
            ),
            ("T[], [A, B]", "Vec<T>, &[T] para los parámetros, tuplas"),
            (
                "T | null, x?: T, ??, ?.",
                "Option<T>, if let, unwrap_or, map",
            ),
            (
                "interface, objeto literal",
                "struct con Debug, Clone, PartialEq",
            ),
            (
                "class, get, set, static",
                "struct, impl, new(), &self o &mut self, consts",
            ),
            ("enum, \"a\" | \"b\"", "enum con Display, match"),
            ("Map, Set, Record<K, V>", "HashMap, HashSet"),
            (
                "throw, try / catch / finally",
                "Result<T, String>, ?, una closure para el bloque try",
            ),
            ("map, filter, reduce, find, some…", "cadenas de iteradores"),
            (
                "for … of, for (;;), while, etiquetas",
                "for x in &v, for i in a..b, loop, 'label",
            ),
            ("switch", "match, con casos agrupados"),
            (
                "async, await",
                "async fn, .await, un pequeño ejecutor para main",
            ),
            (
                "function f<T>(x: T)",
                "fn f<T: Clone + PartialEq>(x: T), cotas deducidas del uso",
            ),
            (
                "console.log, Math, parseInt, …",
                "println!, métodos de f64, pequeñas utilidades",
            ),
            ("clases recursivas", "Option<Box<T>>"),
            (
                "class B extends A, abstract, super, instanceof",
                "structs aplanados, un trait por clase base, Box<dyn ALike> donde se espera un A",
            ),
            (
                "interface Shape { area(): number }, class C implements Shape",
                "un trait por interfaz, impl para cada clase, Box<dyn ShapeLike>",
            ),
            (
                "A | B, [1, \"two\"], uniones discriminadas, typeof",
                "enum con una variante por miembro, match",
            ),
            ("function*, yield, yield*", "JsGenerator<T>, for x in gen"),
            (
                "setTimeout, setInterval, new Promise",
                "un bucle de eventos lanzado por main, JsPromise",
            ),
            (
                "/regex/, test, exec, match, replace, split, lastIndex",
                "JsRegex, un motor con retroceso incluido en el archivo, while let Some(m) = re.exec(&s)",
            ),
            (
                "JSON.stringify, JSON.parse",
                "la JsValue del valor, JsFromJson para el tipo leído",
            ),
            (
                "import, export, import * as ns",
                "un mod por archivo, pub para las exportaciones",
            ),
            (
                "console.log de objetos, arrays, maps",
                "el texto exacto que imprime Node.js",
            ),
            ("any, unknown", "deducidos del uso"),
            (
                "...rest, f(...xs), { ...base, z }, [a, ...rest], ({ x, y }) =>",
                "slices, vec!, literales de struct campo a campo, xs[1..].to_vec()",
            ),
            (
                "new Date(…), getFullYear, toISOString, despues - antes",
                "JsDate en milisegundos (UTC), PartialOrd, diferencias de get_time()",
            ),
            (
                "f?.(), obj?.method(), campos de tipo función",
                "if let Some(f) = …, .map(|o| o.method()), campos Rc<dyn Fn(…)>",
            ),
            (
                "class Stack<T>, interface Page<T>, <T extends HasId>, f<number>(x)",
                "struct Stack<T> + impl<T: …>, cotas encontradas en todo el programa, traits de restricción",
            ),
            (
                "interface Listener<T> { notify(v: T): void }",
                "trait ListenerLike<T>, Box<dyn ListenerLike<String>>",
            ),
        ],
        limits_title: "Qué sigue siendo distinto",
        limits: "Los valores Rust no son referencias JavaScript: un objeto guardado en dos sitios se copia, y modificar una copia no toca la otra (las variables modificadas en closures que les sobreviven se comparten). Los números son `i64` mientras no intervenga un decimal: un entero que desborda provoca un pánico donde JavaScript perdería precisión. Los imports van entre tus propios archivos, no hacia paquetes. Los replacers y revivers de `JSON`, `Symbol`, `Proxy`, `eval`, los prototipos y `arguments` no tienen equivalente en Rust y se rechazan con un mensaje claro.",
    },
    install: Install {
        title: "Instalar",
        lead: "Un crate, ninguna dependencia, un solo binario. Traduce un archivo y los que importa, o pasa un archivo por la entrada estándar.",
        cli: "Línea de comandos",
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
