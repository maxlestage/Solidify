// Generic classes, interfaces and aliases, constraints, explicit type arguments.

class Stack<T> {
  protected items: T[] = [];

  push(item: T): this {
    this.items.push(item);
    return this;
  }

  pop(): T | undefined {
    return this.items.pop();
  }

  peek(): T | undefined {
    return this.items.length > 0 ? this.items[this.items.length - 1] : undefined;
  }

  get size(): number {
    return this.items.length;
  }
}

class NumberStack extends Stack<number> {
  sum(): number {
    return this.items.reduce((a, b) => a + b, 0);
  }
}

class Pair<A, B> {
  constructor(public first: A, public second: B) {}

  swap(): Pair<B, A> {
    return new Pair(this.second, this.first);
  }
}

class Box<T> {
  constructor(public value: T) {}

  static of<U>(value: U): Box<U> {
    return new Box(value);
  }

  map<U>(f: (value: T) => U): Box<U> {
    return new Box(f(this.value));
  }
}

class ListNode<T> {
  next: ListNode<T> | null = null;
  constructor(public value: T) {}
}

class LinkedList<T> {
  private head: ListNode<T> | null = null;
  private count = 0;

  add(value: T): this {
    const node = new ListNode(value);
    node.next = this.head;
    this.head = node;
    this.count++;
    return this;
  }

  toArray(): T[] {
    const out: T[] = [];
    let current = this.head;
    while (current !== null) {
      out.push(current.value);
      current = current.next;
    }
    return out;
  }

  get length(): number {
    return this.count;
  }
}

interface Page<T> {
  items: T[];
  total: number;
}

type Entry<K, V> = [K, V];

interface HasId {
  id: number;
}

interface User extends HasId {
  name: string;
}

function findById<T extends HasId>(items: T[], id: number): T | undefined {
  return items.find((item) => item.id === id);
}

function rename<T extends { name: string }>(x: T, name: string): T {
  x.name = name;
  return x;
}

class Repo<T extends HasId> {
  private items: T[] = [];

  add(item: T): void {
    this.items.push(item);
  }

  ids(): number[] {
    return this.items.map((i) => i.id);
  }
}

function show<T>(x: T): string {
  return `<${x}>`;
}

function identity<T>(x: T): T {
  return x;
}

const numbers = new Stack<number>();
numbers.push(1).push(2).push(3);
console.log(numbers.pop(), numbers.peek(), numbers.size);

const words = new Stack<string>();
words.push("a");
console.log(words.peek(), words);

const total = new NumberStack();
total.push(3).push(4);
console.log(total.sum(), total.size);

const pair = new Pair("x", 3);
const swapped = pair.swap();
console.log(swapped.first + 1, swapped.second, swapped);

const boxed = Box.of(2).map((n) => n * 10).map((n) => `n=${n}`);
console.log(boxed.value, boxed);

const list = new LinkedList<string>();
list.add("a").add("b").add("c");
console.log(list.toArray(), list.length);

const page = JSON.parse('{"items":[1,2],"total":2}') as Page<number>;
console.log(page.items.length, page.total, page);
const empty: Page<string> = { items: [], total: 0 };
console.log(empty);

const entry: Entry<string, number> = ["one", 1];
console.log(entry[0], entry[1]);

const users: User[] = [
  { id: 1, name: "ann" },
  { id: 2, name: "bob" },
];
console.log(findById(users, 2)?.name, findById(users, 5));
console.log(rename({ name: "a" }, "b"));
const repo = new Repo<User>();
repo.add(users[0]);
repo.add(users[1]);
console.log(repo.ids());

console.log(show(1), show("s"), show(users[0]));
console.log(identity<number>(5), identity<string>("x"));
