import summary, { Circle, distance, ORIGIN, type Point } from "./geometry.ts";
import * as geo from "./geometry.ts";
import * as strings from "./util/strings.ts";
import { yell, repeat, Circle as Disc } from "./index.ts";

const unit = new Circle(ORIGIN, 1);
const p: Point = { x: 3, y: 4 };
console.log(summary(unit));
console.log(distance(ORIGIN, p), unit.contains(p), unit.contains({ x: 0.5, y: 0.5 }));
console.log(strings.shout("hello"), strings.SEPARATOR, repeat("ab", 3));
console.log(yell("again"), strings.counted("a"), strings.counted("b"));
const big: Disc = new Disc({ x: 1, y: 1 }, 10);
console.log(unit.area().toFixed(3), big.area().toFixed(1));
console.log(unit);
const c2 = new geo.Circle(geo.ORIGIN, 2);
console.log(c2.area().toFixed(2), geo.distance(geo.ORIGIN, { x: 1, y: 1 }).toFixed(3));
console.log(geo.Circle.unit().radius, c2.shade(), big.shade() === geo.Shade.Dark, geo.Shade.Light);
console.log(big instanceof Disc, unit instanceof geo.Circle);
