fn collatz(mut n: f64) -> i64 {
    let mut steps = 0;
    while n != 1.0 {
        n = if n % 2.0 == 0.0 { n / 2.0 } else { 3.0 * n + 1.0 };
        steps += 1;
    }
    steps
}

fn primes(limit: i64) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::new();
    'outer: for i in 2..=limit {
        let mut j = 2;
        while j * j <= i {
            if i % j == 0 {
                continue 'outer;
            }
            j += 1;
        }
        out.push(i);
    }
    out
}

fn main() {
    let mut i = 0;
    loop {
        i += 3;
        if !(i < 10) {
            break;
        }
    }
    println!("{i} {} {}", collatz(27.0), primes(30).iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" "));
    let mut k = 10;
    while k > 0 {
        'next: {
            if k == 4 {
                break 'next;
            }
            println!("{k}");
        }
        k -= 3;
    }
    let grid = vec![vec![1, 2], vec![3, 4]];
    let mut sum = 0;
    for r in 0..grid.len() {
        for c in 0..grid[r].len() {
            sum += grid[r][c];
        }
    }
    println!("{sum} {} {}", grid[1][0], grid.len());
    let mut count = 0;
    loop {
        count += 1;
        if count > 4 {
            break;
        }
    }
    println!("{count} {} {} {} {} {}", 7 % 3, -7 % 3, 2_i64.pow(10 as u32), 7_i64.div_euclid(2), 7.0 / 2.0);
}
