# Bibliothèque Rust

Le transpileur est un crate Rust, `solidify`, sans dépendance. La ligne de commande et le bac à sable du site sont construits sur la même API.

## L'ajouter

```toml
[dependencies]
solidify = { git = "https://github.com/maxlestage/Solidify" }
```

## Traduire une source

`transpile` traduit une source TypeScript ou JavaScript, donnée sous forme de chaîne :

```rust
let typescript = "const n: number = 6 * 7;\nconsole.log(n);";
let rust: String = solidify::transpile(typescript, &solidify::Options::default())?;
```

Il renvoie le Rust sous forme de `String`, ou une `solidify::Error`. La source ne peut pas importer d'autres fichiers : utilisez `transpile_file` pour un programme sur disque.

## Traduire un programme

`transpile_file` lit le fichier d'entrée situé à `path`, suit ses imports (fichiers relatifs, paquets de `node_modules`, modules de Node) et traduit tout le programme en un seul fichier Rust :

```rust
use std::path::Path;

let rust: String = solidify::transpile_file(Path::new("main.ts"), &solidify::Options::default())?;
std::fs::write("app.rs", rust)?;
```

Son erreur est une `String`, déjà mise en forme avec le fichier, la ligne et la colonne.

## Options

```rust
pub struct Options {
    /// The source file name, mentioned in the header of the output.
    pub file_name: Option<String>,
    /// Omit the header comment.
    pub no_header: bool,
}
```

`Options::default()` écrit un en-tête sans nom de fichier. Pour nommer la source, ou pour omettre l'en-tête :

```rust
let options = solidify::Options {
    file_name: Some("main.ts".to_string()),
    no_header: false,
};
```

## Erreurs

Une `solidify::Error` a un `message` et un `span`, les positions en octets du problème dans la source. Elle implémente `std::error::Error` et `Display` (le message seul), et se met en forme avec la source dont elle vient :

- `error.position(source)` : la ligne et la colonne, à partir de 1 ;
- `error.display(source)` : `line:column: message` ;
- `error.pretty(source)` : le message, l'endroit, la ligne et une marque dessous, comme les affiche la ligne de commande.

```rust
let source = std::fs::read_to_string("app.ts")?;
match solidify::transpile(&source, &solidify::Options::default()) {
    Ok(rust) => println!("{rust}"),
    Err(error) => eprintln!("{}", error.pretty(&source)),
}
```

## Projet

`Project` est ce qu'utilisent les fonctions ci-dessus : les fichiers d'un programme, chargés et analysés. `Project::load(path)` lit un fichier d'entrée et tout ce qu'il importe (une `LoadError` nomme le fichier qui a échoué), `Project::single(name, source)` prend une seule source, et `project.transpile(&options)` traduit. `project.pretty(&error)` met en forme une erreur avec le fichier où elle se trouve, pour les programmes de plusieurs fichiers.

```rust
use solidify::{Options, Project};

let project = Project::load(std::path::Path::new("main.ts"))?;
match project.transpile(&Options::default()) {
    Ok(rust) => std::fs::write("app.rs", rust)?,
    Err(error) => eprintln!("{}", project.pretty(&error)),
}
```

## Dans le navigateur

Le crate se compile en WebAssembly tel quel : le bac à sable de la page d'accueil appelle `transpile` dans votre navigateur, et le paquet npm est la ligne de commande compilée pour WASI.
