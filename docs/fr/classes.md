# Classes et interfaces

Une classe devient un struct et un bloc `impl`. Les interfaces avec des méthodes, et les classes de base, deviennent des traits. Un objet que le programme partage entre deux endroits devient un objet partagé.

## Une classe

Les champs deviennent les champs du struct (les `private` sans `pub`), le constructeur devient `new`, les méthodes prennent `&self` ou `&mut self` selon qu'elles modifient l'objet ou non, et un getter devient une méthode :

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

## Statiques, getters et setters

Un champ `static readonly` est un `const` associé, une méthode statique une fonction associée, et un champ statique qui change est un `static` du module. Un setter devient une méthode `set_` :

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

Un bloc `static { … }` s'exécute avec les autres statics du module.

## Interfaces avec des méthodes

Une interface avec des méthodes devient un trait, nommé d'après elle avec `Like` (`ShapeLike`), implémenté par chaque classe qui implémente l'interface. Une valeur du type de l'interface est une `Box<dyn ShapeLike>` :

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

## Héritage

Rust n'a pas d'héritage : chaque sous-classe devient un struct avec les champs de ses parents, et la classe de base devient un trait, implémenté par chaque sous-classe. Les méthodes `abstract`, `super(…)` et `super.method()` fonctionnent comme en TypeScript, et une valeur du type de la classe de base est une `Box<dyn AnimalLike>` :

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

`instanceof` teste quel struct est une valeur. Les classes écrites à l'ancienne (`function F() { this.x = … }` et `F.prototype.m = …`, ou `const A = class { … }`) sont elles aussi reconstruites en structs et en blocs `impl`.

## Objets à deux endroits

En JavaScript, `const alias = mine` donne deux noms à un même objet. Quand le programme modifie l'objet par un nom et le lit par l'autre, ou compare des objets avec `===`, Solidify en fait un `Shared<T>` : un seul objet (`Rc<RefCell<T>>`) vers lequel pointent les deux noms, comparé par identité.

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

Il affiche `15 true`, comme Node.js. Les objets qui ne sont jamais partagés restent de simples structs.

## Types récursifs

Une classe ou une interface qui se contient elle-même, comme une liste chaînée ou un arbre, contient un `Option<Box<T>>` :

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
