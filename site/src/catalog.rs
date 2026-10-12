//! The table of contents of the documentation: its categories and pages, in reading
//! order. The pages themselves are the Markdown files `docs/{lang}/{slug}.md` at the
//! root of the repository (the same slugs in every language); texts are given in the
//! order English, French, Spanish.

/// A category of pages, with the name of its icon (see `icon()` in `pages.rs`).
pub struct Category {
    pub title: [&'static str; 3],
    pub icon: &'static str,
}

pub struct Page {
    pub slug: &'static str,
    pub category: usize,
    pub title: [&'static str; 3],
    pub summary: [&'static str; 3],
}

pub const CATEGORIES: [Category; 3] = [
    Category {
        title: ["Getting started", "Démarrer", "Primeros pasos"],
        icon: "bolt",
    },
    Category {
        title: ["The translation", "La traduction", "La traducción"],
        icon: "flow",
    },
    Category {
        title: ["Reference", "Référence", "Referencia"],
        icon: "book",
    },
];

pub const PAGES: [Page; 16] = [
    // ----- Getting started
    Page {
        slug: "introduction",
        category: 0,
        title: ["Introduction", "Introduction", "Introducción"],
        summary: [
            "What Solidify is, what it writes, and how to read these pages.",
            "Ce qu'est Solidify, ce qu'il écrit, et comment lire ces pages.",
            "Qué es Solidify, qué escribe y cómo leer estas páginas.",
        ],
    },
    Page {
        slug: "installation",
        category: 0,
        title: ["Installation", "Installation", "Instalación"],
        summary: [
            "npx, the app that works offline, the Rust library, or the sources.",
            "npx, l'app qui marche hors ligne, la bibliothèque Rust, ou les sources.",
            "npx, la app que funciona sin conexión, la biblioteca Rust o el código fuente.",
        ],
    },
    Page {
        slug: "quick-start",
        category: 0,
        title: ["Quick start", "Démarrage rapide", "Inicio rápido"],
        summary: [
            "A first program, from TypeScript to a running Rust binary.",
            "Un premier programme, du TypeScript à un binaire Rust qui tourne.",
            "Un primer programa, de TypeScript a un binario Rust que se ejecuta.",
        ],
    },
    // ----- The translation
    Page {
        slug: "how-it-works",
        category: 1,
        title: ["How it works", "Fonctionnement", "Cómo funciona"],
        summary: [
            "The three passes: parse, infer, generate.",
            "Les trois passes : analyser, inférer, générer.",
            "Las tres pasadas: analizar, inferir, generar.",
        ],
    },
    Page {
        slug: "types",
        category: 1,
        title: ["Types", "Types", "Tipos"],
        summary: [
            "Numbers, strings, arrays, objects, null, unions and any.",
            "Nombres, chaînes, tableaux, objets, null, unions et any.",
            "Números, cadenas, arrays, objetos, null, uniones y any.",
        ],
    },
    Page {
        slug: "ownership",
        category: 1,
        title: [
            "Functions and ownership",
            "Fonctions et propriété",
            "Funciones y propiedad",
        ],
        summary: [
            "Borrowed parameters, moves, clones, closures and generics.",
            "Paramètres empruntés, déplacements, clones, closures et génériques.",
            "Parámetros prestados, movimientos, clones, closures y genéricos.",
        ],
    },
    Page {
        slug: "classes",
        category: 1,
        title: [
            "Classes and interfaces",
            "Classes et interfaces",
            "Clases e interfaces",
        ],
        summary: [
            "Structs, impl blocks, traits, inheritance and shared objects.",
            "Structs, blocs impl, traits, héritage et objets partagés.",
            "Structs, bloques impl, traits, herencia y objetos compartidos.",
        ],
    },
    Page {
        slug: "errors",
        category: 1,
        title: ["Errors", "Erreurs", "Errores"],
        summary: [
            "throw and try become Result; the messages Solidify gives.",
            "throw et try deviennent Result ; les messages de Solidify.",
            "throw y try se vuelven Result; los mensajes de Solidify.",
        ],
    },
    Page {
        slug: "async",
        category: 1,
        title: [
            "Async and timers",
            "Async et minuteries",
            "Async y temporizadores",
        ],
        summary: [
            "Promises, async/await and timers, in Node's order.",
            "Promesses, async/await et minuteries, dans l'ordre de Node.",
            "Promesas, async/await y temporizadores, en el orden de Node.",
        ],
    },
    Page {
        slug: "modules",
        category: 1,
        title: [
            "Modules and programs",
            "Modules et programmes",
            "Módulos y programas",
        ],
        summary: [
            "Several files, imports, top-level code, Node's modules and packages.",
            "Plusieurs fichiers, imports, code de premier niveau, modules de Node et paquets.",
            "Varios archivos, imports, código de nivel superior, módulos de Node y paquetes.",
        ],
    },
    Page {
        slug: "runtime",
        category: 1,
        title: [
            "The JavaScript library",
            "La bibliothèque JavaScript",
            "La biblioteca JavaScript",
        ],
        summary: [
            "console.log, strings, arrays, Map and Set, JSON, RegExp, Date and more.",
            "console.log, chaînes, tableaux, Map et Set, JSON, RegExp, Date et plus.",
            "console.log, cadenas, arrays, Map y Set, JSON, RegExp, Date y más.",
        ],
    },
    // ----- Reference
    Page {
        slug: "cli",
        category: 2,
        title: ["Command line", "Ligne de commande", "Línea de comandos"],
        summary: [
            "Every option of npx solidify-rs, and its exit codes.",
            "Chaque option de npx solidify-rs, et ses codes de sortie.",
            "Cada opción de npx solidify-rs y sus códigos de salida.",
        ],
    },
    Page {
        slug: "library",
        category: 2,
        title: ["Rust library", "Bibliothèque Rust", "Biblioteca Rust"],
        summary: [
            "transpile, transpile_file, Options, Project and errors.",
            "transpile, transpile_file, Options, Project et les erreurs.",
            "transpile, transpile_file, Options, Project y los errores.",
        ],
    },
    Page {
        slug: "mapping",
        category: 2,
        title: [
            "What translates to what",
            "Ce qui se traduit en quoi",
            "Qué se traduce en qué",
        ],
        summary: [
            "Every TypeScript construct and the Rust it becomes.",
            "Chaque construction TypeScript et le Rust qu'elle devient.",
            "Cada construcción de TypeScript y el Rust en que se convierte.",
        ],
    },
    Page {
        slug: "limits",
        category: 2,
        title: ["Limits", "Limites", "Límites"],
        summary: [
            "What stays different from Node.js, and what is refused.",
            "Ce qui reste différent de Node.js, et ce qui est refusé.",
            "Lo que sigue siendo distinto de Node.js y lo que se rechaza.",
        ],
    },
    Page {
        slug: "faq",
        category: 2,
        title: ["FAQ", "FAQ", "Preguntas frecuentes"],
        summary: [
            "Short answers to the questions people ask.",
            "Des réponses courtes aux questions qu'on se pose.",
            "Respuestas cortas a las preguntas habituales.",
        ],
    },
];

/// The page with this slug.
pub fn find(slug: &str) -> Option<&'static Page> {
    PAGES.iter().find(|p| p.slug == slug)
}

/// Its position in reading order.
pub fn index(slug: &str) -> Option<usize> {
    PAGES.iter().position(|p| p.slug == slug)
}

/// The pages of a category, in order.
pub fn pages_of(category: usize) -> impl Iterator<Item = &'static Page> {
    PAGES.iter().filter(move |p| p.category == category)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_unique_and_categories_exist() {
        for (i, page) in PAGES.iter().enumerate() {
            assert_eq!(index(page.slug), Some(i), "{} is listed twice", page.slug);
            assert!(page.category < CATEGORIES.len());
        }
        // Reading order follows the categories.
        assert!(PAGES.windows(2).all(|w| w[0].category <= w[1].category));
    }
}
