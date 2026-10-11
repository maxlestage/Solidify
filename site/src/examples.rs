//! The programs shown in the playground: the test cases of the transpiler, so that what
//! the site shows is what the tests check.

use crate::routes::Lang;

pub struct Example {
    pub id: &'static str,
    pub title_en: &'static str,
    pub title_fr: &'static str,
    pub source: &'static str,
}

impl Example {
    pub fn title(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::En => self.title_en,
            Lang::Fr => self.title_fr,
        }
    }
}

macro_rules! case {
    ($id:literal, $en:literal, $fr:literal) => {
        Example {
            id: $id,
            title_en: $en,
            title_fr: $fr,
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
    case!("fib", "Recursion & loops", "Récursion et boucles"),
    case!("classes", "Classes & interfaces", "Classes et interfaces"),
    case!("objects", "Nullable values", "Valeurs nullables"),
    case!("errors", "Errors", "Erreurs"),
    case!("arrays", "Arrays", "Tableaux"),
    case!("collections", "Maps & sets", "Maps et sets"),
    case!("linked", "Linked list", "Liste chaînée"),
    case!("closures", "Closures & generics", "Closures et génériques"),
    case!("enums", "Enums & switch", "Enums et switch"),
    case!("strings", "Strings", "Chaînes"),
];

pub fn example(id: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|e| e.id == id)
}
