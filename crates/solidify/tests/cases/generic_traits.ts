// Generic interfaces with methods and generic base classes, held as `dyn` traits.

interface Container<T> {
  get(): T;
  size(): number;
}

interface Named<T> {
  name: string;
  value: T;
}

interface Tagged extends Named<number> {
  tag: string;
}

class Cell implements Container<number> {
  constructor(private v: number) {}
  get(): number {
    return this.v;
  }
  size(): number {
    return 1;
  }
}

class Many implements Container<string> {
  constructor(private items: string[]) {}
  get(): string {
    return this.items.join("+");
  }
  size(): number {
    return this.items.length;
  }
}

function describe<T>(c: Container<T>): string {
  return `${c.get()} (${c.size()})`;
}

interface Listener<T> {
  notify(value: T): void;
}

class Logger implements Listener<string> {
  lines: string[] = [];
  notify(value: string): void {
    this.lines.push(`log: ${value}`);
  }
}

class Counter implements Listener<string> {
  count = 0;
  notify(value: string): void {
    this.count += value.length;
  }
}

class Emitter<T> {
  private listeners: Listener<T>[] = [];
  on(l: Listener<T>): void {
    this.listeners.push(l);
  }
  emit(value: T): void {
    for (const l of this.listeners) {
      l.notify(value);
    }
  }
  get size(): number {
    return this.listeners.length;
  }
}

class Recorder<T> implements Listener<T> {
  seen: T[] = [];
  notify(value: T): void {
    this.seen.push(value);
  }
}

abstract class Shape<T> {
  constructor(public meta: T) {}
  abstract area(): number;
  describe(): string {
    return `${this.area().toFixed(1)} ${this.meta}`;
  }
}

class Square extends Shape<string> {
  constructor(private side: number) {
    super("square");
  }
  area(): number {
    return this.side * this.side;
  }
}

class Circle extends Shape<string> {
  constructor(private r: number) {
    super("circle");
  }
  area(): number {
    return Math.PI * this.r * this.r;
  }
}

const t: Tagged = { name: "n", value: 3, tag: "x" };
console.log(t.value + 1, t);
console.log(describe(new Cell(4)), describe(new Many(["a", "b"])));
const cells: Container<number>[] = [new Cell(1), new Cell(2)];
console.log(cells.map((c) => c.get()));

const emitter = new Emitter<string>();
emitter.on(new Logger());
emitter.on(new Counter());
emitter.emit("hello");
emitter.emit("hi");
console.log(emitter.size);

const recorder = new Recorder<number>();
recorder.notify(3);
recorder.notify(4);
console.log(recorder.seen);

const shapes: Shape<string>[] = [new Square(2), new Circle(1)];
for (const s of shapes) {
  console.log(s.describe());
}
