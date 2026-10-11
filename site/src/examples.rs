//! The programs shown in the playground: the test cases of the transpiler, so that what
//! the site shows is what the tests check.

use crate::routes::Lang;

pub struct Example {
    pub id: &'static str,
    pub title_en: &'static str,
    pub title_fr: &'static str,
    pub title_es: &'static str,
    pub source: &'static str,
}

impl Example {
    pub fn title(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::En => self.title_en,
            Lang::Fr => self.title_fr,
            Lang::Es => self.title_es,
        }
    }
}

macro_rules! case {
    ($id:literal, $en:literal, $fr:literal, $es:literal) => {
        Example {
            id: $id,
            title_en: $en,
            title_fr: $fr,
            title_es: $es,
            source: include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../crates/solidify/tests/cases/",
                $id,
                ".ts"
            )),
        }
    };
}

pub const EXAMPLES: &[Example] = &[
    case!(
        "fib",
        "Recursion & loops",
        "Récursion et boucles",
        "Recursión y bucles"
    ),
    case!(
        "classes",
        "Classes & interfaces",
        "Classes et interfaces",
        "Clases e interfaces"
    ),
    case!(
        "objects",
        "Nullable values",
        "Valeurs nullables",
        "Valores anulables"
    ),
    case!("errors", "Errors", "Erreurs", "Errores"),
    case!("arrays", "Arrays", "Tableaux", "Arrays"),
    case!("collections", "Maps & sets", "Maps et sets", "Maps y sets"),
    case!("linked", "Linked list", "Liste chaînée", "Lista enlazada"),
    case!(
        "closures",
        "Closures & generics",
        "Closures et génériques",
        "Closures y genéricos"
    ),
    case!(
        "enums",
        "Enums & switch",
        "Enums et switch",
        "Enums y switch"
    ),
    case!("strings", "Strings", "Chaînes", "Cadenas"),
    case!(
        "narrowing",
        "Trees & null checks",
        "Arbres et tests de null",
        "Árboles y comprobaciones de null"
    ),
    case!(
        "plain",
        "Plain JavaScript",
        "JavaScript sans types",
        "JavaScript sin tipos"
    ),
    case!(
        "unions",
        "Unions & typeof",
        "Unions et typeof",
        "Uniones y typeof"
    ),
    case!("inheritance", "Inheritance", "Héritage", "Herencia"),
    case!(
        "interfaces",
        "Interfaces as traits",
        "Interfaces en traits",
        "Interfaces como traits"
    ),
    case!(
        "generics",
        "Generic classes",
        "Classes génériques",
        "Clases genéricas"
    ),
    case!(
        "generic_traits",
        "Generic interfaces",
        "Interfaces génériques",
        "Interfaces genéricas"
    ),
    case!(
        "missing",
        "undefined & null",
        "undefined et null",
        "undefined y null"
    ),
    case!(
        "spreads",
        "Spreads & rest",
        "Spreads et rest",
        "Spreads y rest"
    ),
    case!("generators", "Generators", "Générateurs", "Generadores"),
    case!(
        "regex",
        "Regular expressions",
        "Expressions régulières",
        "Expresiones regulares"
    ),
    case!("json", "JSON", "JSON", "JSON"),
    case!("dates", "Dates", "Dates", "Fechas"),
    case!(
        "callbacks",
        "Callbacks & optional calls",
        "Callbacks et appels optionnels",
        "Callbacks y llamadas opcionales"
    ),
    case!(
        "timers",
        "Timers & promises",
        "Minuteurs et promesses",
        "Temporizadores y promesas"
    ),
    case!("printing", "console.log", "console.log", "console.log"),
    case!(
        "parsing",
        "Numbers & any",
        "Nombres et any",
        "Números y any"
    ),
];

pub fn example(id: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|e| e.id == id)
}
