function greet(name) {
  return "hello " + name;
}
function shout(s) {
  return s.toUpperCase();
}
module.exports = { greet, loud: shout };
