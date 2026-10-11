fn main() {
    let words = vec!["pear".to_string(), "apple".to_string(), "fig".to_string(), "banana".to_string()];
    let lengths = words.iter().map(|w| w.chars().count() as i64).collect::<Vec<_>>();
    let long = words.iter().filter(|w| w.chars().count() > 3).cloned().collect::<Vec<_>>();
    let total = lengths.iter().fold(0, |acc, &n| acc + n);
    println!("{} {} {total}", lengths.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","), long.join(" "));
    let mut sorted = words.clone();
    sorted.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    println!("{}", sorted.join(" "));
    let mut nums = vec![5, 3, 9, 1];
    nums.push(7);
    nums.sort_by(|a, b| a.cmp(b));
    println!("{} {} {} {}", nums.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" "), nums.contains(&9), js_index_of_item(&nums, &3), nums.iter().copied().max().unwrap_or(i64::MIN));
    let found = words.iter().find(|w| w.starts_with("b")).cloned();
    if let Some(found) = &found {
        println!("{}", found.to_uppercase());
    }
}

/// `list.indexOf(item)`: the position, or -1.
fn js_index_of_item<T: PartialEq>(v: &[T], item: &T) -> i64 {
    v.iter().position(|x| x == item).map_or(-1, |i| i as i64)
}
