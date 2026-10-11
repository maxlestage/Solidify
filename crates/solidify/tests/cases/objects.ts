interface User {
  name: string;
  age: number;
  email?: string;
}

const users: User[] = [
  { name: "Ann", age: 31, email: "ann@example.com" },
  { name: "Bob", age: 17 },
  { name: "Cid", age: 45 },
];

function adults(list: User[]): User[] {
  return list.filter((u) => u.age >= 18);
}

function oldest(list: User[]): User | null {
  let best: User | null = null;
  for (const u of list) {
    if (best === null || u.age > best.age) {
      best = u;
    }
  }
  return best;
}

const grown = adults(users);
console.log(grown.length, grown.map((u) => u.name).join(", "));
const top = oldest(users);
if (top !== null) {
  console.log(`${top.name} is the oldest (${top.age})`);
  console.log(top.email ?? "no email");
}
const copy = { ...users[0], age: 32 };
console.log(copy.name, copy.age, users[0].age);
const point = { x: 1, y: 2 };
const moved = { x: point.x + 1, y: point.y };
console.log(moved.x, moved.y, point.x);
const { name, age } = users[1];
console.log(name, age);
