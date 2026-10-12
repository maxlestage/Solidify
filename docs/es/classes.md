# Clases e interfaces

Una clase se convierte en un struct y un bloque `impl`. Las interfaces con métodos, y las clases base, se convierten en traits. Un objeto que el programa comparte entre dos lugares se convierte en un objeto compartido.

## Una clase

Los campos se convierten en los campos del struct (los `private`, sin `pub`), el constructor se convierte en `new`, los métodos reciben `&self` o `&mut self` según modifiquen o no el objeto, y un getter se convierte en un método:

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

## Estáticos, getters y setters

Un campo `static readonly` es una `const` asociada, un método estático una función asociada, y un campo estático que cambia es un `static` del módulo. Un setter se convierte en un método `set_`:

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

Un bloque `static { … }` se ejecuta con los demás estáticos del módulo.

## Interfaces con métodos

Una interfaz con métodos se convierte en un trait, con su nombre seguido de `Like` (`ShapeLike`), implementado por cada clase que implementa la interfaz. Un valor del tipo de la interfaz es un `Box<dyn ShapeLike>`:

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

## Herencia

Rust no tiene herencia: cada subclase se convierte en un struct con los campos de sus padres, y la clase base se convierte en un trait, implementado por cada subclase. Los métodos `abstract`, `super(…)` y `super.method()` funcionan como en TypeScript, y un valor del tipo de la clase base es un `Box<dyn AnimalLike>`:

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

`instanceof` comprueba qué struct es un valor. Las clases escritas a la antigua (`function F() { this.x = … }` y `F.prototype.m = …`, o `const A = class { … }`) también se reconstruyen como structs y bloques `impl`.

## Objetos en dos lugares

En JavaScript, `const alias = mine` crea dos nombres para un mismo objeto. Cuando el programa modifica el objeto a través de un nombre y lo lee a través del otro, o compara objetos con `===`, Solidify lo convierte en un `Shared<T>`: un único objeto (`Rc<RefCell<T>>`) al que apuntan ambos nombres, comparado por identidad.

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

Imprime `15 true`, como Node.js. Los objetos que nunca se comparten siguen siendo structs normales.

## Tipos recursivos

Una clase o interfaz que se contiene a sí misma, como una lista enlazada o un árbol, contiene `Option<Box<T>>`:

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
