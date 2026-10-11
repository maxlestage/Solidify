class Node {
  next: Node | null = null;
  constructor(public value: number) {}
}

class List {
  head: Node | null = null;
  size = 0;

  push(value: number): void {
    const node = new Node(value);
    if (this.head === null) {
      this.head = node;
    } else {
      let current = this.head;
      while (current.next !== null) {
        current = current.next;
      }
      current.next = node;
    }
    this.size++;
  }

  sum(): number {
    let total = 0;
    let current = this.head;
    while (current !== null) {
      total += current.value;
      current = current.next;
    }
    return total;
  }

  first(): number | null {
    if (this.head === null) return null;
    return this.head.value;
  }
}

const list = new List();
list.push(3);
list.push(4);
list.push(5);
console.log(list.size, list.sum(), list.first());
const empty = new List();
const f = empty.first();
console.log(f === null ? "empty" : `first ${f}`);
