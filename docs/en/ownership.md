# Functions and ownership

TypeScript lets any part of a program hold any value; Rust wants to know who owns each one. Solidify decides it from what the program does: it borrows when it can, moves when a value is no longer needed, and clones only when two places keep the same value.

## Borrowed parameters

A function that only reads a parameter borrows it: a `string` becomes a `&str`, an array a `&[T]`, an object a `&T`. The caller lends its value and keeps it.

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

`ada` is pushed into `users` and then still read by `welcome`: the array keeps a clone, and `welcome` borrows the original.

## Changed parameters

A function that changes a parameter takes it as `&mut`, and the caller's variable becomes `mut`:

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

## Moves

A value used for the last time is moved, not cloned. Without the `welcome(ada)` of the first example, `ada` goes into the array as it is:

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

Strings and structs are cloned only where the program really keeps two copies; numbers and booleans are copied, as in Rust.

## Closures

An arrow function is a Rust closure. A closure that changes a variable it captured shares it with the function that made it (`Rc<RefCell<…>>`), so that each call sees the last value:

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

A function taking a callback takes `impl Fn(…)`; a field or an array holding functions holds `Rc<dyn Fn(…)>`.

## Generics

A type parameter becomes a Rust type parameter, with the bounds its uses need (`Clone` here, because `best` keeps an item):

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

A parameter without annotation that nothing constrains becomes a type parameter the same way. Generic classes and interfaces (`class Stack<T>`, `<T extends HasId>`) get the bounds found across the whole program.

## Objects in two places

Most objects have one owner. When the program stores the same object in two places and changes it through one of them, or compares objects with `===`, the object becomes shared: see [classes and interfaces](classes.md#objects-in-two-places).
