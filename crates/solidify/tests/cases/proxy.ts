// Proxies: reads and writes through traps, the target shared with the proxy.

const user = { name: "Ada", age: 36 };
const validated = new Proxy(user, {
  set(target, key, value) {
    if (key === "age" && typeof value !== "number") {
      throw new TypeError("age must be a number");
    }
    target[key] = value;
    return true;
  },
});
validated.age = 37;
console.log(validated.age, user.age, user);

const prefix = "read:";
const logged = new Proxy({ x: 1, y: 2 }, {
  get(t, k) {
    console.log(prefix, k);
    return Reflect.get(t, k);
  },
});
console.log(logged.x + logged.y);

const defaults = new Proxy({} as Record<string, number>, {
  get: (t, k) => (k in t ? t[k] : 0),
});
console.log(defaults.anything, defaults.other);

const target = JSON.parse('{"name":"ada"}');
const shout = new Proxy(target, {
  get(t, key) {
    return key in t ? t[key] : `no ${key}`;
  },
  set(t, key, value) {
    t[key] = typeof value === "string" ? value.toUpperCase() : value;
    return true;
  },
  has: (t, key) => key.startsWith("n"),
});
shout.city = "paris";
console.log(shout.name, shout.age, shout.city, target.city);
console.log("name" in shout, "city" in shout, Reflect.ownKeys(target));

const record = JSON.parse('{"a":1,"b":2}');
delete record.a;
console.log(record);
const scores: Record<string, number> = { x: 1, y: 2 };
delete scores["x"];
console.log(scores);
