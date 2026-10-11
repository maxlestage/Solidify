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
    match n % 3 {
        0 => {
            kind = "fizz".to_string();
        }
        1 => {
            kind = "one".to_string();
        }
        _ => {
            kind = "other".to_string();
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
    println!("{} {}", first_negative(&[3, 1, -2, 5]), first_negative(&[1, 2]));
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
    let mut counts = vec![("a".to_string(), 1)].into_iter().collect::<JsMap<_, _>>();
    for key in counts.keys().cloned().collect::<Vec<_>>() {
        counts.insert(key.clone(), counts.get(&key).copied().unwrap() + 1);
    }
    println!("{evens} {} {}", counts.get("a").copied().as_ref().map_or("undefined".to_string(), |v| v.to_string()), { let mut v = vec![3, 1, 2]; v.sort_by(|a, b| a.cmp(b)); v }.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(""));
}

/// A `Map`: a hash map that keeps its keys in insertion order, like JavaScript.
#[derive(Clone, Debug)]
struct JsMap<K, V> {
    entries: Vec<(K, V)>,
    index: std::collections::HashMap<K, usize>,
}

#[allow(dead_code)]
impl<K: std::hash::Hash + Eq + Clone, V> JsMap<K, V> {
    fn new() -> Self {
        JsMap { entries: Vec::new(), index: std::collections::HashMap::new() }
    }
    fn len(&self) -> usize {
        self.entries.len()
    }
    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    fn position<Q: ?Sized + std::hash::Hash + Eq>(&self, key: &Q) -> Option<usize>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.index.get(key).copied()
    }
    fn get<Q: ?Sized + std::hash::Hash + Eq>(&self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.position(key).map(|i| &self.entries[i].1)
    }
    fn get_mut<Q: ?Sized + std::hash::Hash + Eq>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.position(key).map(|i| &mut self.entries[i].1)
    }
    fn contains_key<Q: ?Sized + std::hash::Hash + Eq>(&self, key: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
    {
        self.index.contains_key(key)
    }
    /// `map.set(key, value)`: an existing key keeps its position.
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.index.get(&key) {
            Some(&i) => Some(std::mem::replace(&mut self.entries[i].1, value)),
            None => {
                self.index.insert(key.clone(), self.entries.len());
                self.entries.push((key, value));
                None
            }
        }
    }
    fn remove<Q: ?Sized + std::hash::Hash + Eq>(&mut self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
    {
        let i = self.index.remove(key)?;
        let (_, value) = self.entries.remove(i);
        for (_, pos) in self.index.iter_mut() {
            if *pos > i {
                *pos -= 1;
            }
        }
        Some(value)
    }
    fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }
    fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|(k, _)| k)
    }
    fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }
    fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries.iter_mut().map(|(_, v)| v)
    }
    fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
    fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.entries.iter_mut().map(|(k, v)| (&*k, v))
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> Default for JsMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: std::hash::Hash + Eq + Clone, V: PartialEq> PartialEq for JsMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> FromIterator<(K, V)> for JsMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = JsMap::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

impl<K: std::hash::Hash + Eq + Clone, V, const N: usize> From<[(K, V); N]> for JsMap<K, V> {
    fn from(entries: [(K, V); N]) -> Self {
        entries.into_iter().collect()
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> Extend<(K, V)> for JsMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> IntoIterator for JsMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a, K: std::hash::Hash + Eq + Clone, V> IntoIterator for &'a JsMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::iter::Map<std::slice::Iter<'a, (K, V)>, fn(&'a (K, V)) -> (&'a K, &'a V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter().map(|(k, v)| (k, v))
    }
}

impl<K: std::hash::Hash + Eq + Clone + std::borrow::Borrow<Q>, V, Q: ?Sized + std::hash::Hash + Eq>
    std::ops::Index<&Q> for JsMap<K, V>
{
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("no entry found for key")
    }
}

