# Classes and interfaces

A class becomes a struct and an `impl` block. Interfaces with methods, and base classes, become traits. An object that the program shares between two places becomes a shared object.

## A class

Fields become the struct's fields (`private` ones without `pub`), the constructor becomes `new`, methods take `&self` or `&mut self` depending on whether they change the object, and a getter becomes a method:

```ts
class Counter {
  private count = 0;

  constructor(public name: string) {}

  increment(): void {
    this.count++;
  }

  get value(): number {
    return this.count;
  }
}

const c = new Counter("clicks");
c.increment();
c.increment();
console.log(`${c.name}: ${c.value}`);
```
```rust
#[derive(Debug, Clone, PartialEq)]
struct Counter {
    pub name: String,
    count: i64,
}

impl Counter {
    pub fn new(name: String) -> Self {
        Self { name, count: 0 }
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }

    pub fn value(&self) -> i64 {
        self.count
    }
}

fn main() {
    let mut c = Counter::new("clicks".to_string());
    c.increment();
    c.increment();
    println!("{}: {}", c.name, c.value());
}
```

## Statics, getters and setters

A `static readonly` field is an associated `const`, a static method an associated function, and a static field that changes is a `static` of the module. A setter becomes a `set_` method:

```ts
class Temperature {
  static readonly FREEZING = 0;
  private static created = 0;

  constructor(private celsius: number) {
    Temperature.created++;
  }

  get fahrenheit(): number {
    return this.celsius * 9 / 5 + 32;
  }

  set fahrenheit(f: number) {
    this.celsius = (f - 32) * 5 / 9;
  }

  static count(): number {
    return Temperature.created;
  }
}

const t = new Temperature(100);
t.fahrenheit = 50;
console.log(t.fahrenheit, Temperature.count(), Temperature.FREEZING);
```
```rust fragment
// …
static TEMPERATURE_CREATED: LazyLock<JsStatic<i64>> = LazyLock::new(|| JsStatic::new(0));

#[derive(Debug, Clone, PartialEq)]
struct Temperature {
    celsius: f64,
}

impl Temperature {
    pub const FREEZING: i64 = 0;

    pub fn new(celsius: f64) -> Self {
        let mut this = Self { celsius };
        (*TEMPERATURE_CREATED.write()) += 1;
        this
    }

    pub fn fahrenheit(&self) -> f64 {
        self.celsius * 9.0 / 5.0 + 32.0
    }

    pub fn set_fahrenheit(&mut self, f: f64) {
        self.celsius = (f - 32.0) * 5.0 / 9.0;
    }

    pub fn count() -> i64 {
        (*TEMPERATURE_CREATED.read())
    }
}

// …
```

A `static { … }` block runs with the other statics of the module.

## Interfaces with methods

An interface with methods becomes a trait, named after it with `Like` (`ShapeLike`), implemented by every class that implements the interface. A value of the interface's type is a `Box<dyn ShapeLike>`:

```ts
interface Shape {
  area(): number;
}

class Square implements Shape {
  constructor(private side: number) {}
  area(): number {
    return this.side * this.side;
  }
}

function total(shapes: Shape[]): number {
  return shapes.reduce((sum, s) => sum + s.area(), 0);
}

console.log(total([new Square(2), new Square(3)]));
```
```rust
#[derive(Debug, Clone, PartialEq)]
struct Square {
    side: i64,
}

impl Square {
    pub fn new(side: i64) -> Self {
        Self { side }
    }

    pub fn area(&self) -> i64 {
        self.side * self.side
    }
}

fn total(shapes: &[Box<dyn ShapeLike>]) -> i64 {
    shapes.iter().fold(0, |sum, s| sum + s.area())
}

fn main() {
    println!("{}", total(&[Box::new(Square::new(2)) as Box<dyn ShapeLike>, Box::new(Square::new(3)) as Box<dyn ShapeLike>]));
}

/// `Shape` and the classes implementing it.
trait ShapeLike: std::fmt::Debug {
    fn area(&self) -> i64;
    fn clone_shape(&self) -> Box<dyn ShapeLike>;
}

impl Clone for Box<dyn ShapeLike> {
    fn clone(&self) -> Self {
        self.clone_shape()
    }
}

impl ShapeLike for Square {
    fn area(&self) -> i64 {
        Square::area(self)
    }

    fn clone_shape(&self) -> Box<dyn ShapeLike> {
        Box::new(self.clone())
    }
}
```

## Inheritance

Rust has no inheritance: each subclass becomes a struct with the fields of its parents, and the base class becomes a trait, implemented by every subclass. `abstract` methods, `super(…)` and `super.method()` work as in TypeScript, and a value of the base class's type is a `Box<dyn AnimalLike>`:

```ts
abstract class Animal {
  constructor(public name: string) {}
  abstract sound(): string;
  speak(): string {
    return `${this.name} says ${this.sound()}`;
  }
}

class Dog extends Animal {
  sound(): string {
    return "woof";
  }
}

class Cat extends Animal {
  sound(): string {
    return "meow";
  }
}

const pets: Animal[] = [new Dog("Rex"), new Cat("Tom")];
for (const pet of pets) {
  console.log(pet.speak());
}
```
```rust fragment
// …
#[derive(Debug, Clone, PartialEq)]
struct Dog {
    pub name: String,
}

impl Dog {
    pub fn new(name: String) -> Self {
        let base = Animal::new(name);
        Self { name: base.name }
    }

    pub fn sound(&self) -> String {
        "woof".to_string()
    }

    pub fn speak(&self) -> String {
        format!("{} says {}", self.name, self.sound())
    }
}

// …
fn main() {
    let pets = vec![Box::new(Dog::new("Rex".to_string())) as Box<dyn AnimalLike>, Box::new(Cat::new("Tom".to_string())) as Box<dyn AnimalLike>];
    for pet in &pets {
        println!("{}", pet.speak());
    }
}

/// `Animal` and its subclasses.
trait AnimalLike: std::fmt::Debug {
    fn name(&self) -> &String;
    fn sound(&self) -> String;
    fn speak(&self) -> String;
    fn clone_animal(&self) -> Box<dyn AnimalLike>;
}
// …
```

`instanceof` tests which struct a value is. Classes written the old way (`function F() { this.x = … }` and `F.prototype.m = …`, or `const A = class { … }`) are rebuilt as structs and `impl` blocks too.

## Objects in two places

In JavaScript, `const alias = mine` makes two names for one object. When the program changes the object through one name and reads it through the other, or compares objects with `===`, Solidify makes it a `Shared<T>`: one object (`Rc<RefCell<T>>`) that both names point to, compared by identity.

```ts
class Account {
  constructor(public balance: number) {}
}

const mine = new Account(10);
const alias = mine;
alias.balance += 5;
console.log(mine.balance, mine === alias);
```
```rust fragment
#[derive(Debug, Clone, PartialEq)]
struct Account {
    pub balance: i64,
}

impl Account {
    pub fn new(balance: i64) -> Self {
        Self { balance }
    }
}

fn main() {
    let mine = Shared::new(Account::new(10));
    let mut alias = mine.clone();
    alias.borrow_mut().balance += 5;
    println!("{} {}", mine.borrow().balance, &mine == &alias);
}

// …
```

It prints `15 true`, as Node.js does. Objects that are never shared stay plain structs.

## Recursive types

A class or interface that contains itself, such as a linked list or a tree, holds `Option<Box<T>>`:

```ts
class ListNode {
  constructor(public value: number, public next: ListNode | null = null) {}
}

function sum(node: ListNode | null): number {
  return node === null ? 0 : node.value + sum(node.next);
}

console.log(sum(new ListNode(1, new ListNode(2, new ListNode(3)))));
```
```rust
#[derive(Debug, Clone, PartialEq)]
struct ListNode {
    pub value: i64,
    pub next: Option<Box<ListNode>>,
}

impl ListNode {
    pub fn new(value: i64, next: Option<Box<ListNode>>) -> Self {
        Self { value, next }
    }
}

fn sum(node: Option<&ListNode>) -> i64 {
    if let Some(node) = node { node.value + sum(node.next.as_deref()) } else { 0 }
}

fn main() {
    println!("{}", sum(Some(&ListNode::new(1, Some(Box::new(ListNode::new(2, Some(Box::new(ListNode::new(3, None))))))))));
}
```
