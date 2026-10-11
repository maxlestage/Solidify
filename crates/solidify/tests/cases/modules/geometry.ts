export interface Point {
  x: number;
  y: number;
}

export class Circle {
  constructor(
    public center: Point,
    public radius: number,
  ) {}

  area(): number {
    return Math.PI * this.radius * this.radius;
  }

  contains(p: Point): boolean {
    return distance(this.center, p) <= this.radius;
  }
}

export function distance(a: Point, b: Point): number {
  return Math.sqrt(square(a.x - b.x) + square(a.y - b.y));
}

// Not exported: only this file uses it.
function square(n: number): number {
  return n * n;
}

export const ORIGIN: Point = { x: 0, y: 0 };

export default function describe(c: Circle): string {
  return `circle r=${c.radius} at (${c.center.x}, ${c.center.y})`;
}

console.log("geometry loaded");
