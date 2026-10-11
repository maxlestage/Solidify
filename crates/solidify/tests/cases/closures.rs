fn make_counter() -> impl FnMut() -> i64 {
    let mut count = 0;
    move || {
        count += 1;
        count
    }
}

fn apply(f: impl Fn(i64) -> i64, value: i64) -> i64 {
    f(value)
}

fn first<T: Clone>(items: &[T]) -> T {
    items[0].clone()
}

fn same<T: PartialEq>(a: T, b: T) -> bool {
    &a == &b
}

fn main() {
    let double = |x: i64| x * 2;
    let mut next = make_counter();
    next();
    next();
    println!("{} {} {}", next(), apply(double, 21), apply(|x| x + 1, 1));
    let greet = |name: String, excited: bool| {
        let base = format!("Hello, {name}");
        if excited { format!("{base}!") } else { base }
    };
    println!("{} {}", greet("Ada".to_string(), false), greet("Bob".to_string(), true));
    println!("{} {} {} {}", first(&vec![7, 8]), first(&vec!["a".to_string(), "b".to_string()]), same(1, 1), same("x".to_string(), "y".to_string()));
    let mut hits = 0;
    vec![1, 2, 3].into_iter().for_each(|n| {
        if n % 2 == 1 {
            hits += 1;
        }
    });
    println!("{hits}");
}
