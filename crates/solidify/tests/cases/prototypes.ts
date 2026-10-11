// Older JavaScript: constructor functions and prototypes, fields set in the constructor,
// `arguments`, arguments left out.

function Person(name, age) {
  this.name = name;
  this.age = age;
}
Person.prototype.greet = function () {
  return "Hi " + this.name;
};
Person.prototype.birthday = function () {
  this.age += 1;
};
function Student(name, age, school) {
  Person.call(this, name, age);
  this.school = school;
}
Student.prototype = Object.create(Person.prototype);
Student.prototype.constructor = Student;
Student.prototype.describe = function () {
  return this.greet() + " from " + this.school;
};
const p = new Person("Ann", 30);
p.birthday();
const s = new Student("Bob", 20, "MIT");
console.log(p.greet(), p.age, s.describe(), s instanceof Person);

function sum() {
  let total = 0;
  for (let i = 0; i < arguments.length; i++) total += arguments[i];
  return total;
}
function first(a, b) {
  return arguments.length + a;
}
console.log(sum(1, 2, 3), sum(), first(1, 2), first(5));

class Point {
  constructor(x, y) {
    this.x = x;
    this.y = y;
  }
  norm() { return Math.sqrt(this.x * this.x + this.y * this.y); }
}
console.log(new Point(3, 4).norm());
