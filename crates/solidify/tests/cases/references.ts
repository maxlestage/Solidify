// Objects, arrays and maps held in several places are one value, like in JavaScript;
// `===` compares identity.

class Counter {
  count = 0;
  constructor(public name: string) {}
  inc() {
    this.count++;
  }
}
const a = new Counter("a");
const b = a;
b.inc();
console.log(a.count, b.count, a === b, a === new Counter("a"));
const counters = [a];
counters[0].inc();
console.log(a.count, counters.indexOf(a), counters.includes(new Counter("a")));

const point = { x: 1 };
const alias = point;
alias.x = 5;
console.log(point.x);

const nums = [1, 2];
const same = nums;
same.push(3);
const holder = { items: nums };
holder.items.push(4);
const scores = new Map<string, number>();
const scores2 = scores;
scores2.set("a", 1);
console.log(nums, same.length, holder.items.length, scores.size);

class TreeNode {
  children: TreeNode[] = [];
  parent: TreeNode | null = null;
  constructor(public name: string) {}
  add(child: TreeNode): TreeNode {
    child.parent = this;
    this.children.push(child);
    return child;
  }
  path(): string {
    return this.parent ? this.parent.path() + "/" + this.name : this.name;
  }
}
const root = new TreeNode("root");
const docs = root.add(new TreeNode("docs"));
const readme = docs.add(new TreeNode("readme"));
readme.name = "README";
console.log(readme.path(), root.children[0].children[0].name, docs.parent === root);

class Account {
  constructor(public owner: string, public balance: number) {}
  deposit(n: number) {
    this.balance += n;
  }
}
const accounts = [new Account("ann", 10), new Account("bob", 5)];
const byOwner = new Map<string, Account>();
for (const acc of accounts) byOwner.set(acc.owner, acc);
byOwner.get("ann")!.deposit(100);
const richest = accounts.reduce((best, x) => (x.balance > best.balance ? x : best));
richest.deposit(1);
console.log(accounts[0].balance, accounts.indexOf(richest));
const visits = new Map<Account, number>();
visits.set(accounts[1], 3);
console.log(visits.get(accounts[1]), visits.has(new Account("bob", 5)));

class Builder {
  parts: string[] = [];
  add(p: string): Builder {
    this.parts.push(p);
    return this;
  }
}
const builder = new Builder();
const chained = builder.add("x").add("y");
chained.add("z");
console.log(builder.parts.join(""), chained === builder);

const registry: Item[] = [];
class Item {
  constructor(public name: string) {
    registry.push(this);
  }
}
const item = new Item("first");
item.name = "renamed";
console.log(registry[0].name, registry.length);

// A cycle is shown the way Node shows it.
class Ring {
  next: Ring | null = null;
  constructor(public value: number) {}
}
const r1 = new Ring(1);
const r2 = new Ring(2);
r1.next = r2;
r2.next = r1;
console.log(r1.next!.next === r1, r1);
