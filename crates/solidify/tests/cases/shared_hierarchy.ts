// Objects of a class hierarchy held in several places: one object, seen as its base
// class or as itself.

abstract class Shape {
  moves = 0;
  constructor(public name: string) {}
  abstract area(): number;
  move() { this.moves++; }
  describe(): string { return `${this.name} ${this.area().toFixed(1)} moved ${this.moves}`; }
}
class Circle extends Shape {
  constructor(public r: number) { super("circle"); }
  area(): number { return Math.PI * this.r * this.r; }
  grow() { this.r += 1; }
}
class Sq extends Shape {
  constructor(public side: number) { super("square"); }
  area(): number { return this.side * this.side; }
  describe(): string { return "[" + super.describe() + "]"; }
}
const shapes: Shape[] = [new Circle(1), new Sq(2)];
const picked = shapes[0];
picked.move();
for (const s of shapes) {
  if (s instanceof Circle) {
    s.grow();
    console.log("circle r", s.r);
  }
}
console.log(shapes.map((s) => s.describe()));
console.log(picked === shapes[0], shapes.indexOf(picked), shapes[1] instanceof Sq);
const c = new Circle(3);
const asShape: Shape = c;
asShape.move();
c.grow();
console.log(c.moves, asShape.describe(), shapes);

class Animal2 {
  energy = 10;
  constructor(public name: string) {}
  eat() { this.energy += 5; }
  speak(): string { return `${this.name} (${this.energy})`; }
}
class Dog extends Animal2 {
  bark(): string { return "woof"; }
  speak(): string { return `${this.name} barks (${this.energy})`; }
}
const zoo: Animal2[] = [new Dog("rex"), new Animal2("cat")];
const first = zoo[0];
first.eat();
console.log(zoo[0].speak(), first.speak());
const d = new Dog("fido");
const d2 = d;
d2.eat();
console.log(d.energy, d === d2);
