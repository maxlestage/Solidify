use std::collections::{HashMap, HashSet};

fn main() {
    // Maps, sets, records and tuples.
    let mut stock: HashMap<String, i64> = HashMap::new();
    stock.insert("apple".to_string(), 3);
    stock.insert("pear".to_string(), 0);
    stock.insert("fig".to_string(), 12);
    if stock.contains_key("apple") {
        println!("apples: {}", stock.get("apple").copied().as_ref().map_or("undefined".to_string(), |v| v.to_string()));
    }
    let missing = stock.get("kiwi").copied();
    println!("{}", if let Some(missing) = missing { "kiwi!".to_string() } else { "no kiwi".to_string() });
    let mut in_stock = 0;
    for (name, &count) in &stock {
        if count > 0 {
            in_stock += 1;
        }
        if name.as_str() == "fig" {
            println!("figs: {count}");
        }
    }
    println!("in stock: {in_stock} kinds: {}", stock.len());
    let mut seen: HashSet<i64> = HashSet::new();
    for n in vec![1, 2, 2, 3, 3, 3] {
        seen.insert(n);
    }
    println!("{} {} {}", seen.len(), seen.contains(&2), seen.contains(&9));
    let mut ages = HashMap::from([("ann".to_string(), 31), ("bob".to_string(), 27)]);
    ages.insert("cid".to_string(), 45);
    let mut names = ages.keys().cloned().collect::<Vec<_>>();
    names.sort();
    for n in &names {
        println!("{n} {}", ages[n.as_str()]);
    }
    let pair = ("x".to_string(), 10);
    let (label, value) = pair.clone();
    println!("{label} {} {}", value * 2, pair.0);
}
