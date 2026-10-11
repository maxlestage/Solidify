// console.log shows objects, arrays, maps and sets the way Node does.
interface Point {
  x: number;
  y: number;
}

class Dog {
  constructor(public name: string, public age: number, public nickname?: string) {}
}

type Status = "active" | "idle";

interface User {
  id: number;
  tags: string[];
  status: Status;
  home: Point | null;
}

const points: Point[] = [{ x: 1, y: 2 }, { x: 3, y: 4 }];
console.log(points, points[0], [1, 2, 3], ["a", "it's"], []);
console.log(new Dog("Rex", 3), [new Dog("a", 1), new Dog("b", 2, "bee")]);
const user: User = { id: 7, tags: ["x", "y"], status: "active", home: null };
console.log(user, { nested: { deep: { deeper: { deepest: 1 } } } });
const scores = new Map<string, number>();
scores.set("zoe", 10);
scores.set("adam", 7);
scores.set("maya", 9);
scores.delete("adam");
scores.set("adam", 8);
const seen = new Set<number>([3, 1, 3, 2]);
console.log(scores, seen, new Map<string, number>(), new Set<string>());
console.log("Scores:", [...scores.keys()], [...scores.values()], [...seen]);
for (const [name, score] of scores) {
  console.log(`${name} => ${score}`);
}
const numbers = Array.from({ length: 30 }, (_, i) => i * 100);
console.log(numbers);
const words = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa", "lambda"];
console.log(words);
const matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
console.log({ matrix }, [[[["deep"]]]]);
console.log(`${[1, 2, 3]} ${String(points[1])} ${matrix}`, [1.5, 2.25, -0.5], [true, false]);
const record: Record<string, number[]> = { b: [2], a: [1, 1] };
console.log(record, Object.keys(record), Object.entries(record));
const found = points.find((p) => p.x > 2);
console.log(found, points.find((p) => p.x > 10), scores.get("zoe"), scores.get("nobody"));
let line = "items: ";
line += points.map((p) => p.x);
console.log(line, [1, 2].join(", "), [[1, 2], [3]].join(";"), [1, 2, 3].toString());
