// Classes extending classes: fields and methods are inherited, overrides dispatch
// dynamically through a trait, super calls reach the base class.
abstract class Shape {
  constructor(public name: string) {}

  abstract area(): number;

  describe(): string {
    return `${this.name} with area ${this.area().toFixed(2)}`;
  }
}

class Circle extends Shape {
  constructor(public radius: number) {
    super("circle");
  }

  area(): number {
    return Math.PI * this.radius * this.radius;
  }
}

class Rect extends Shape {
  constructor(public width: number, public height: number) {
    super("rect");
  }

  area(): number {
    return this.width * this.height;
  }

  describe(): string {
    return `${super.describe()} (${this.width}x${this.height})`;
  }
}

class Square extends Rect {
  constructor(side: number) {
    super(side, side);
    this.name = "square";
  }
}

class Animal {
  sounds = 0;

  constructor(public name: string) {}

  speak(): string {
    this.sounds++;
    return `${this.name} makes a sound`;
  }

  static kingdom(): string {
    return "animalia";
  }
}

class Dog extends Animal {
  constructor(name: string, public tricks: string[]) {
    super(name);
  }

  speak(): string {
    this.sounds++;
    return `${this.name} barks`;
  }

  learn(trick: string): void {
    this.tricks.push(trick);
  }
}

class Puppy extends Dog {
  speak(): string {
    return `${this.name} yaps`;
  }
}

function loudest(animals: Animal[]): string {
  let best = animals[0];
  for (const a of animals) {
    if (a.sounds > best.sounds) {
      best = a;
    }
  }
  return best.name;
}

const shapes: Shape[] = [new Circle(1), new Rect(2, 3), new Square(2)];
for (const s of shapes) {
  console.log(s.describe());
}
const total = shapes.reduce((sum, s) => sum + s.area(), 0);
console.log(total.toFixed(3), shapes.length);

const rex = new Dog("Rex", []);
rex.learn("sit");
rex.learn("roll");
console.log(rex.speak(), rex.tricks.length, rex.tricks.join("+"), Dog.kingdom());
const pets: Animal[] = [new Animal("Generic"), rex, new Puppy("Bit", ["beg"])];
for (const p of pets) {
  console.log(p.speak());
  if (p instanceof Dog) {
    p.learn("fetch");
    console.log(`${p.name} knows ${p.tricks.length} tricks`);
  }
  if (p instanceof Puppy) {
    console.log("a puppy!");
  }
}
pets[0].name = "Gen";
console.log(loudest(pets), pets[0].name, pets.map((p) => p.sounds).join(","));
console.log(pets[1], shapes[0]);
