// Getters, setters, static members, string enums and a class with a toString.
enum Level {
  Low = "low",
  High = "high",
}

class Thermostat {
  static readonly MAX = 30;
  private _target = 20;
  readonly name: string;
  history: number[] = [];

  constructor(name: string) {
    this.name = name;
  }

  get target(): number {
    return this._target;
  }

  set target(value: number) {
    this._target = Math.min(value, Thermostat.MAX);
    this.history.push(this._target);
  }

  get level(): Level {
    return this._target >= 25 ? Level.High : Level.Low;
  }

  toString(): string {
    return `${this.name}@${this._target}`;
  }

  static fromDefault(): Thermostat {
    return new Thermostat("default");
  }
}

const t = new Thermostat("hall");
t.target = 24;
t.target = 40;
console.log(t.target, t.level, t.history.length, Thermostat.MAX);
console.log(`${t}`, String(t), Thermostat.fromDefault().name);
const level: Level = Level.High;
console.log(level === Level.High, level);
