# La bibliothèque JavaScript

Un programme utilise plus que le langage : `console.log`, les méthodes des chaînes et des tableaux, `Map`, `JSON`, les expressions régulières, les dates. Chacun devient l'équivalent propre à Rust quand il en existe un, et sinon une fonction utilitaire écrite dans la sortie, qui se comporte comme Node.js.

## console.log

`console.log` affiche exactement ce qu'affiche Node.js : nombres, chaînes, objets, tableaux, maps et sets, avec les mêmes espaces et les mêmes guillemets. Les arguments sont séparés par des espaces ; `console.error` affiche sur la sortie d'erreur standard.

```ts
const user = { name: "Ada", langs: ["en", "fr"], admin: true };
console.log(user);
console.log([1, 2, 3].map((n) => n * 2));
console.log(new Map([["a", 1]]), new Set([1, 2]));
```
```rust fragment
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub name: String,
    pub langs: Vec<String>,
    pub admin: bool,
}

fn main() {
    let user = User { name: "Ada".to_string(), langs: vec!["en".to_string(), "fr".to_string()], admin: true };
    println!("{}", js_inspect(&user.js_value()));
    println!("{}", js_inspect(&(vec![1, 2, 3].into_iter().map(|n| n * 2).collect::<Vec<_>>()).js_value()));
    println!("{} {}", js_inspect(&(vec![("a".to_string(), 1)].into_iter().collect::<JsMap<_, _>>()).js_value()), js_inspect(&(vec![1, 2].into_iter().collect::<JsSet<_>>()).js_value()));
}
// …
```

Il affiche `{ name: 'Ada', langs: [ 'en', 'fr' ], admin: true }`, puis `[ 2, 4, 6 ]`, puis `Map(1) { 'a' => 1 } Set(2) { 1, 2 }`.

## Chaînes

Les méthodes des chaînes correspondent à celles de Rust (`toUpperCase` est `to_uppercase`, `trim` est `trim`, `split` est `split`, `includes` est `contains`, `startsWith` est `starts_with`…), et celles qui manquent à Rust sont des fonctions utilitaires : `charAt`, `padStart`, `slice` avec des indices négatifs, `localeCompare`. Les indices et les longueurs comptent les caractères comme le fait JavaScript.

## Tableaux

`map`, `filter`, `reduce`, `find`, `some`, `every`, `forEach`, `flatMap`, `includes`, `indexOf`, `join` deviennent des chaînes d'itérateurs ou des méthodes de `Vec` ; `push`, `pop`, `splice`, `sort`, `reverse` modifient le vecteur sur place. `sort()` sans comparateur trie les nombres comme des chaînes, comme le fait JavaScript. Le spread (`[...a, ...b]`) construit un nouveau vecteur.

## Map et Set

`Map` et `Set` deviennent `JsMap` et `JsSet` : des tables de hachage qui gardent leur ordre d'insertion, comme celles de JavaScript, quand le programme peut l'observer (en les parcourant ou en les affichant).

```ts
const counts = new Map<string, number>();
for (const word of "the cat and the hat".split(" ")) {
  counts.set(word, (counts.get(word) ?? 0) + 1);
}
for (const [word, n] of counts) {
  console.log(word, n);
}
```
```rust fragment
fn main() {
    let mut counts: JsMap<String, i64> = JsMap::new();
    for word in "the cat and the hat".split(" ").map(String::from).collect::<Vec<String>>() {
        counts.insert(word.clone(), counts.get(&word).copied().unwrap_or(0) + 1);
    }
    for (word, &n) in &counts {
        println!("{word} {n}");
    }
}
// …
```

`Record<K, V>` et les objets utilisés comme dictionnaires sont aussi des maps.

## JSON

`JSON.parse` lit le texte dans le type qu'attend le programme (`JsFromJson`), et lève les messages de `SyntaxError` de V8 quand il n'y arrive pas. `JSON.stringify` écrit n'importe quelle valeur, avec le replacer et l'indentation quand ils sont donnés.

```ts
interface Config {
  port: number;
  hosts: string[];
}

const config: Config = JSON.parse('{"port": 8080, "hosts": ["a", "b"]}');
console.log(config.port + 1, config.hosts.length);
console.log(JSON.stringify({ ok: true, port: config.port }));
```
```rust fragment
#[derive(Debug, Clone, PartialEq)]
struct Config {
    pub port: f64,
    pub hosts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub ok: bool,
    pub port: f64,
}

fn main() -> Result<(), String> {
    let config = js_json_parse_as::<Config>("{\"port\": 8080, \"hosts\": [\"a\", \"b\"]}")?;
    println!("{} {}", js_number_text(config.port + 1.0), config.hosts.len());
    println!("{}", js_json_stringify(&(Object { ok: true, port: config.port }).js_value(), ""));
    Ok(())
}
// …
```

Un nombre lu depuis du JSON est un `f64`, comme en JavaScript.

## Expressions régulières

Un littéral d'expression régulière devient un `JsRegex` : la syntaxe de JavaScript (groupes, groupes nommés, lookahead et lookbehind, drapeaux `g`, `i`, `m`, `s`, `u`, `y`), reconnue par un moteur à retour arrière écrit dans le fichier. `test`, `exec`, `match`, `matchAll`, `replace`, `replaceAll`, `split` et `search` fonctionnent dessus.

```ts
const line = "2026-10-12 ERROR disk full";
const match = /^(\d{4})-(\d{2})-(\d{2}) (\w+)/.exec(line);
if (match !== null) {
  console.log(match[4], match[1]);
}
console.log("a1b22c333".replace(/\d+/g, "#"));
```
```rust fragment
const LINE: &str = "2026-10-12 ERROR disk full";

fn main() {
    let r#match = JsRegex::new("^(\\d{4})-(\\d{2})-(\\d{2}) (\\w+)", "").exec(LINE);
    if let Some(r#match) = &r#match {
        println!("{} {}", r#match.get(4), r#match.get(1));
    }
    println!("{}", JsRegex::new("\\d+", "g").replace("a1b22c333", "#"));
}
// …
```

## Dates

`new Date(…)` devient un `JsDate` : des millisecondes depuis 1970, en UTC. Ses getters, `toISOString`, `getTime`, `Date.now()` et la soustraction de deux dates fonctionnent comme en JavaScript.

```ts
const start = new Date("2026-01-31T00:00:00Z");
const later = new Date(start.getTime() + 24 * 60 * 60 * 1000);
console.log(later.toISOString(), later.getMonth());
```
```rust fragment
fn main() {
    let start = JsDate::parse("2026-01-31T00:00:00Z");
    let later = JsDate::from_ms(start.get_time() + 24 * 60 * 60 * 1000);
    println!("{} {}", later.to_iso_string(), later.get_month());
}
// …
```

> [!NOTE]
> Les dates sont lues et affichées en UTC. `Date.UTC` n'est pas pris en charge : écrivez plutôt la date sous forme de chaîne ISO.

## Générateurs

Une fonction génératrice (`function*`) renvoie un `JsGenerator<T>`, un itérateur : `for … of` l'exécute, `yield*` délègue à un autre générateur.

```ts
function* range(from: number, to: number) {
  for (let i = from; i < to; i++) {
    yield i;
  }
}

let sum = 0;
for (const n of range(1, 5)) {
  sum += n;
}
console.log(sum);
```
```rust fragment
fn range(from: i64, to: i64) -> JsGenerator<i64> {
    JsGenerator::new(move |y| {
        for i in from..to {
            y.yield_(i);
        }
    })
}

fn main() {
    let mut sum = 0;
    for n in range(1, 5) {
        sum += n;
    }
    println!("{sum}");
}
// …
```

## Math et nombres

`Math`, ce sont les méthodes de `f64` de Rust (`Math.sqrt(x)` est `x.sqrt()`, `Math.max` est `max`), et `Math.random` un petit générateur écrit dans le fichier. `toFixed`, `toString(radix)`, `Number.isInteger` et les autres méthodes des nombres affichent comme le fait JavaScript.

## Code dynamique

Ce que seul JavaScript sait faire tourne quand même : `Proxy` et `Reflect` (`JsProxy` et ses pièges), `Symbol` (`JsSymbol`), `arguments` (un paramètre rest). `eval` et `new Function` intègrent le code quand il est connu au moment de la traduction, et sinon l'exécutent dans un petit interpréteur écrit dans le fichier, plus lent que le code traduit.
