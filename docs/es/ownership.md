# Funciones y propiedad

TypeScript deja que cualquier parte de un programa guarde cualquier valor; Rust quiere saber quién es el dueño de cada uno. Solidify lo decide a partir de lo que hace el programa: presta cuando puede, mueve cuando un valor ya no hace falta y clona solo cuando dos lugares guardan el mismo valor.

## Parámetros prestados

Una función que solo lee un parámetro lo toma prestado: un `string` se convierte en un `&str`, un array en un `&[T]`, un objeto en un `&T`. Quien llama presta su valor y lo conserva.

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

`ada` se añade a `users` y después `welcome` todavía lo lee: el array guarda un clon, y `welcome` toma prestado el original.

## Parámetros modificados

Una función que modifica un parámetro lo recibe como `&mut`, y la variable de quien llama se vuelve `mut`:

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

## Movimientos

Un valor que se usa por última vez se mueve, no se clona. Sin el `welcome(ada)` del primer ejemplo, `ada` entra en el array tal cual:

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

Las cadenas y los structs solo se clonan donde el programa guarda de verdad dos copias; los números y los booleanos se copian, como en Rust.

## Closures

Una función flecha es una closure de Rust. Una closure que modifica una variable que ha capturado la comparte con la función que la creó (`Rc<RefCell<…>>`), para que cada llamada vea el último valor:

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

Una función que recibe un callback recibe `impl Fn(…)`; un campo o un array que contiene funciones contiene `Rc<dyn Fn(…)>`.

## Genéricos

Un parámetro de tipo se convierte en un parámetro de tipo de Rust, con los límites que necesitan sus usos (aquí `Clone`, porque `best` guarda un elemento):

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

Un parámetro sin anotación al que nada restringe se convierte en un parámetro de tipo de la misma manera. Las clases e interfaces genéricas (`class Stack<T>`, `<T extends HasId>`) reciben los límites hallados en todo el programa.

## Objetos en dos lugares

La mayoría de los objetos tienen un solo dueño. Cuando el programa guarda el mismo objeto en dos lugares y lo modifica a través de uno de ellos, o compara objetos con `===`, el objeto pasa a ser compartido: consulta [clases e interfaces](classes.md#objetos-en-dos-lugares).
