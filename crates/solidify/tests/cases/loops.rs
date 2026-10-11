use std::collections::HashMap;

fn search(grid: &[Vec<i64>], target: i64) -> Option<(i64, i64)> {
    'outer: for r in 0..grid.len() {
        for c in 0..grid[r].len() {
            if grid[r][c] == target {
                return Some((r as i64, c as i64));
            }
            if grid[r][c] > target {
                continue 'outer;
            }
        }
    }
    None
}

fn describe(n: i64) -> String {
    let mut kind = "".to_string();
    'switch: {
        match n % 3 {
        0 => {
            kind = "fizz".to_string();
        }
        1 => {
            {
                kind = "one".to_string();
                break 'switch;
            }
        }
        _ => {
            kind = "other".to_string();
        }
        }
    }
    kind
}

fn first_negative(xs: &[i64]) -> i64 {
    let mut i = 0;
    loop {
        if i >= xs.len() as i64 {
            return -1;
        }
        if xs[i as usize] < 0 {
            break;
        }
        i += 1;
    }
    i
}

fn main() {
    let grid = vec![vec![1, 3, 5], vec![2, 4, 6]];
    let hit = search(&grid, 4);
    if let Some(hit) = hit {
        let (r, c) = hit;
        println!("found at {r},{c}");
    }
    println!("{} {} {} {}", search(&grid, 7).is_none(), describe(9), describe(10), describe(11));
    println!("{} {}", first_negative(&vec![3, 1, -2, 5]), first_negative(&vec![1, 2]));
    let mut evens = 0;
    let mut i = 10;
    while i >= 0 {
        evens += 1;
        if i == 4 {
            break;
        }
        i -= 2;
    }
    let names = vec!["ann".to_string(), "bob".to_string()];
    for (i, n) in names.iter().enumerate() {
        println!("{i} {n}");
    }
    let mut counts = vec![("a".to_string(), 1)].into_iter().collect::<HashMap<_, _>>();
    for key in counts.keys().cloned().collect::<Vec<_>>() {
        counts.insert(key.clone(), counts.get(&key).copied().unwrap() + 1);
    }
    println!("{evens} {} {}", counts.get("a").copied().as_ref().map_or("undefined".to_string(), |v| v.to_string()), { let mut v = vec![3, 1, 2]; v.sort_by(|a, b| a.cmp(b)); v }.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(""));
}
