# Fonctions et propriété

TypeScript laisse n'importe quelle partie d'un programme tenir n'importe quelle valeur ; Rust veut savoir qui possède chacune. Solidify le décide d'après ce que fait le programme : il emprunte quand il le peut, déplace quand une valeur n'est plus nécessaire, et ne clone que quand deux endroits gardent la même valeur.

## Paramètres empruntés

Une fonction qui ne fait que lire un paramètre l'emprunte : un `string` devient un `&str`, un tableau un `&[T]`, un objet un `&T`. L'appelant prête sa valeur et la garde.

```ts
interface User {
  name: string;
}

function welcome(user: User): string {
  return `Welcome, ${user.name}`;
}

const users: User[] = [];
const ada = { name: "Ada" };
users.push(ada);
console.log(welcome(ada));
console.log(users.length);
```
```rust
#[derive(Debug, Clone, PartialEq)]
struct User {
    pub name: String,
}

fn welcome(user: &User) -> String {
    format!("Welcome, {}", user.name)
}

fn main() {
    let mut users: Vec<User> = Vec::new();
    let ada = User { name: "Ada".to_string() };
    users.push(ada.clone());
    println!("{}", welcome(&ada));
    println!("{}", users.len());
}
```

`ada` est ajouté à `users`, puis encore lu par `welcome` : le tableau garde un clone, et `welcome` emprunte l'original.

## Paramètres modifiés

Une fonction qui modifie un paramètre le prend en `&mut`, et la variable de l'appelant devient `mut` :

```ts
function addAll(target: number[], extra: number[]): void {
  for (const x of extra) {
    target.push(x);
  }
}

const list = [1, 2];
addAll(list, [3, 4]);
console.log(list.length);
```
```rust
fn add_all(target: &mut Vec<i64>, extra: &[i64]) {
    for &x in extra {
        target.push(x);
    }
}

fn main() {
    let mut list = vec![1, 2];
    add_all(&mut list, &[3, 4]);
    println!("{}", list.len());
}
```

## Déplacements

Une valeur utilisée pour la dernière fois est déplacée, pas clonée. Sans le `welcome(ada)` du premier exemple, `ada` entre dans le tableau tel quel :

```ts
interface User {
  name: string;
}

const users: User[] = [];
const ada = { name: "Ada" };
users.push(ada);
console.log(users.length);
```
```rust
#[derive(Debug, Clone, PartialEq)]
struct User {
    pub name: String,
}

fn main() {
    let mut users: Vec<User> = Vec::new();
    let ada = User { name: "Ada".to_string() };
    users.push(ada);
    println!("{}", users.len());
}
```

Les chaînes et les structs ne sont clonés que là où le programme garde vraiment deux copies ; les nombres et les booléens sont copiés, comme en Rust.

## Closures

Une fonction fléchée est une closure Rust. Une closure qui modifie une variable qu'elle a capturée la partage avec la fonction qui l'a créée (`Rc<RefCell<…>>`), pour que chaque appel voie la dernière valeur :

```ts
function makeCounter(): () => number {
  let count = 0;
  return () => {
    count += 1;
    return count;
  };
}

const next = makeCounter();
next();
console.log(next());
```
```rust
use std::cell::RefCell;
use std::rc::Rc;

fn make_counter() -> impl FnMut() -> i64 {
    let count = Rc::new(RefCell::new(0));
    { let count = count.clone(); move || {
        (*count.borrow_mut()) += 1;
        (*count.borrow())
    } }
}

fn main() {
    let mut next = make_counter();
    next();
    println!("{}", next());
}
```

Une fonction qui prend un callback prend un `impl Fn(…)` ; un champ ou un tableau qui contient des fonctions contient des `Rc<dyn Fn(…)>`.

## Génériques

Un paramètre de type devient un paramètre de type Rust, avec les bornes dont ses usages ont besoin (`Clone` ici, parce que `best` garde un élément) :

```ts
function largest<T>(items: T[], better: (a: T, b: T) => boolean): T {
  let best = items[0];
  for (const item of items) {
    if (better(item, best)) {
      best = item;
    }
  }
  return best;
}

console.log(largest([3, 9, 4], (a, b) => a > b));
console.log(largest(["pear", "fig"], (a, b) => a.length > b.length));
```
```rust
fn largest<T: Clone>(items: &[T], better: impl Fn(T, T) -> bool) -> T {
    let mut best = items[0].clone();
    for item in items {
        if better(item.clone(), best.clone()) {
            best = item.clone();
        }
    }
    best
}

fn main() {
    println!("{}", largest(&[3, 9, 4], |a, b| a > b));
    println!("{}", largest(&["pear".to_string(), "fig".to_string()], |a, b| a.chars().count() > b.chars().count()));
}
```

Un paramètre sans annotation que rien ne contraint devient un paramètre de type de la même façon. Les classes et interfaces génériques (`class Stack<T>`, `<T extends HasId>`) reçoivent les bornes trouvées dans tout le programme.

## Objets à deux endroits

La plupart des objets ont un seul propriétaire. Quand le programme range le même objet à deux endroits et le modifie par l'un d'eux, ou compare des objets avec `===`, l'objet devient partagé : voir [classes et interfaces](classes.md#objets-à-deux-endroits).
