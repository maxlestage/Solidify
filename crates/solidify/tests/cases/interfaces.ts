// An interface with methods is a trait; the classes implementing it are its values.
interface Named {
  name: string;
}

interface Shape extends Named {
  area(): number;
  scale(k: number): void;
}

class Sq implements Shape {
  name = "square";
  constructor(private side: number) {}
  area(): number {
    return this.side * this.side;
  }
  scale(k: number): void {
    this.side *= k;
  }
}

class Ci implements Shape {
  name = "circle";
  constructor(private r: number) {}
  area(): number {
    return Math.PI * this.r * this.r;
  }
  scale(k: number): void {
    this.r *= k;
  }
  perimeter(): number {
    return 2 * Math.PI * this.r;
  }
}

function total(list: Shape[]): number {
  return list.reduce((a, s) => a + s.area(), 0);
}

function describe(s: Shape): string {
  return `${s.name}: ${s.area().toFixed(2)}`;
}

const shapes: Shape[] = [new Sq(2), new Ci(1)];
for (const sh of shapes) {
  console.log(sh.name, sh.area().toFixed(2));
}
console.log(total(shapes).toFixed(2), describe(new Sq(3)));
shapes[0].scale(2);
console.log(shapes[0].area(), shapes.length, shapes[1]);
const circle = new Ci(2);
console.log(circle.perimeter().toFixed(3), describe(circle));

// A class with the right fields stands in for an interface without `implements`.
interface Point {
  x: number;
  y: number;
}

class Pixel {
  constructor(
    public x: number,
    public y: number,
    public color: string,
  ) {}
}

function manhattan(p: Point): number {
  return Math.abs(p.x) + Math.abs(p.y);
}

const origin: Point = { x: 0, y: 0 };
const px = new Pixel(-3, 4, "red");
console.log(manhattan(origin), manhattan(px), manhattan({ x: 1, y: 1 }));
const points: Point[] = [origin, px, { x: 5, y: 5 }];
console.log(points.map((p) => p.x + p.y));
