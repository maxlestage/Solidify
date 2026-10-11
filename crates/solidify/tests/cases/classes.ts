interface Point {
  x: number;
  y: number;
  label?: string;
}

class Counter {
  count = 0;
  constructor(public name: string) {}
  increment(): void {
    this.count++;
  }
  get label(): string {
    return `${this.name}: ${this.count}`;
  }
}

function describe(p: Point): string {
  if (p.label !== undefined) {
    return p.label;
  }
  return `(${p.x}, ${p.y})`;
}

const c = new Counter("clicks");
c.increment();
c.increment();
console.log(c.label);
const points: Point[] = [{ x: 1, y: 2 }, { x: 3, y: 4, label: "far" }];
for (const p of points) {
  console.log(describe(p));
}
