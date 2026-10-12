# Erreurs

Deux sortes d'erreurs se rencontrent ici : celles que le programme lève pendant qu'il tourne, qui deviennent des `Result` de Rust, et celles que Solidify signale quand il ne peut pas traduire un programme, avec le fichier, la ligne et la colonne.

## throw devient Result

Une fonction qui lève une erreur, ou qui en appelle une qui le fait hors d'un `try`, renvoie `Result<T, String>` : `throw new Error("…")` devient `return Err(…)`, et ses appelants transmettent l'erreur avec `?`. Un bloc `try` devient une closure dont `catch` reçoit l'erreur, sous forme de message :

```ts
function withdraw(balance: number, amount: number): number {
  if (amount > balance) {
    throw new Error(`only ${balance} left`);
  }
  return balance - amount;
}

try {
  console.log(withdraw(10, 4));
  console.log(withdraw(10, 40));
} catch (e) {
  console.log(`refused: ${e.message}`);
}
```
```rust
fn withdraw(balance: i64, amount: i64) -> Result<i64, String> {
    if amount > balance {
        return Err(format!("only {balance} left"));
    }
    Ok(balance - amount)
}

fn main() {
    let result_: Result<(), String> = (|| {
        println!("{}", withdraw(10, 4)?);
        println!("{}", withdraw(10, 40)?);
        Ok(())
    })();
    if let Err(e) = result_ {
        println!("refused: {e}");
    }
}
```

Une erreur que rien n'attrape remonte jusqu'à `main`, qui la renvoie : le programme s'arrête avec le code de sortie 1 et affiche le message sur la sortie d'erreur standard (`Error: "only 10 left"`), là où Node.js afficherait une pile d'appels.

## Classes d'erreurs

Une classe qui étend `Error` garde ses champs, son message et son nom à travers le `Result` ; `instanceof` dans `catch` la retrouve :

```ts
class NotFound extends Error {
  constructor(public key: string) {
    super(`no ${key}`);
    this.name = "NotFound";
  }
}

function lookup(key: string): string {
  if (key !== "a") {
    throw new NotFound(key);
  }
  return "found";
}

try {
  lookup("b");
} catch (e) {
  if (e instanceof NotFound) {
    console.log(`${e.name}: ${e.key}`);
  }
}
```
```rust fragment
#[derive(Debug, Clone, PartialEq)]
struct NotFound {
    pub key: String,
    pub message: String,
    pub name: String,
}

impl NotFound {
    pub fn new(key: String) -> Self {
        Self { key: key.clone(), message: format!("no {key}"), name: "NotFound".to_string() }
    }
}

fn lookup(key: &str) -> Result<String, String> {
    if key != "a" {
        return Err(js_error_encode(&NotFound::new(key.to_string()).js_value(), "NotFound,Error"));
    }
    Ok("found".to_string())
}

fn main() {
    let result_: Result<(), String> = (|| {
        lookup("b")?;
        Ok(())
    })();
    if let Err(e) = result_ {
        if let Some(e) = js_error_as::<NotFound>(&e, "NotFound") {
            println!("{}: {}", e.name, e.key);
        }
    }
}

// …
```

`JSON.parse` lève les mêmes messages de `SyntaxError` que V8, donc un programme qui les affiche affiche le même texte.

## Les messages de Solidify

Quand un programme ne peut pas être traduit, Solidify dit pourquoi et où : le message, puis le fichier, la ligne et la colonne, la ligne elle-même et une marque sous l'endroit. `npx solidify-rs --check` ne fait que les signaler, sans rien écrire.

### Une valeur qui peut être null

Lire une valeur qui peut être `null` ou `undefined` est une erreur, comme dans le mode strict de TypeScript :

```ts error
const names: string[] = ["Ada"];
const first = names.find((n) => n.startsWith("A"));
console.log(first.length);
```
```text
error: `first` may be null here (string | null): test it with `if (… !== null)`, or use `?.` or `!`
  --> app.ts:3:13
   |
  3| console.log(first.length);
   |             ^^^^^^^^^^^^
```

Testez-la d'abord (`if (first !== undefined)`), donnez une valeur par défaut (`first ?? ""`), lisez-la avec `?.`, ou affirmez-la avec `!` quand vous en savez plus.

### Un type qui ne convient pas

Une variable garde un seul type. Lui donner une valeur d'un autre type est une erreur :

```ts error
let count = 0;
count = "many";
```
```text
error: expected number (integer), found string
  --> app.ts:2:9
   |
  2| count = "many";
   |         ^^^^^^
```

Utilisez deux variables, ou déclarez l'union que vous voulez (`let count: number | string = 0`).

### Ce qui n'est pas pris en charge

Quelques constructions sont refusées, chacune avec un message qui la nomme :

```ts error
async function* ticks() {
  yield 1;
}
```
```text
error: async generators are not supported
  --> app.ts:1:7
   |
  1| async function* ticks() {
   |       ^^^^^^^^^^^^^^^^^^^
```

Les [limites](limits.md) les listent toutes.

## Dans la bibliothèque Rust

`solidify::transpile` renvoie une `solidify::Error` : son `message`, et un `span` (des positions en octets dans la source). `pretty(source)` la met en forme comme ci-dessus ; la page [Bibliothèque Rust](library.md#erreurs) montre comment.
