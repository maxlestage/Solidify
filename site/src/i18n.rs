//! Interface texts in English and French.

use crate::routes::Lang;

pub struct Dict {
    pub lang_name: &'static str,
    pub locale: &'static str,
    pub other_lang: &'static str,
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
    }
}

static EN: Dict = Dict {
    lang_name: "English",
    locale: "en_US",
    other_lang: "Français",
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
            ("30", "programs checked against Node.js"),
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
    other_lang: "English",
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
            ("30", "programmes vérifiés contre Node.js"),
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
