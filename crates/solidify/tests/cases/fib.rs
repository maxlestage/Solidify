/// Classic recursion and a counted loop.
fn fib(n: i64) -> i64 {
    if n < 2 {
        return n;
    }
    fib(n - 1) + fib(n - 2)
}

fn main() {
    let mut total = 0;
    for i in 0..10 {
        total += fib(i);
    }
    println!("sum of the first ten: {total}");
    println!("fib(20) = {}", fib(20));
}
