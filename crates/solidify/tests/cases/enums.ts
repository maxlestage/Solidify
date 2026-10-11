enum Color {
  Red,
  Green,
  Blue,
}

type Direction = "up" | "down" | "left" | "right";

function hex(c: Color): string {
  switch (c) {
    case Color.Red:
      return "#f00";
    case Color.Green:
      return "#0f0";
    case Color.Blue:
      return "#00f";
  }
}

function step(d: Direction): number {
  switch (d) {
    case "up":
    case "right":
      return 1;
    default:
      return -1;
  }
}

function classify(n: number): string {
  switch (n) {
    case 0:
      return "zero";
    case 1:
    case 2:
      return "small";
    default:
      return "big";
  }
}

console.log(hex(Color.Green), step("left"), step("up"), classify(2), classify(7));
let dir: Direction = "down";
if (dir === "down") dir = "up";
console.log(dir, Color.Blue);
const colors = [Color.Red, Color.Blue];
console.log(colors.length, colors.includes(Color.Blue));

enum Level {
  Off,
  Low,
  High = 5,
}
const level: Level = Level.High;
console.log(Level[level], Level[Level.Low], Level[0], Level[7]);
