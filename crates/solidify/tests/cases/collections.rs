fn main() {
    // Maps, sets, records and tuples.
    let mut stock: JsMap<String, i64> = JsMap::new();
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
    let mut seen: JsSet<i64> = JsSet::new();
    for n in vec![1, 2, 2, 3, 3, 3] {
        seen.insert(n);
    }
    println!("{} {} {}", seen.len(), seen.contains(&2), seen.contains(&9));
    let mut ages = JsMap::from([("ann".to_string(), 31), ("bob".to_string(), 27)]);
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


/// A `Set`: a hash set that keeps its values in insertion order, like JavaScript.
#[derive(Clone, Debug)]
struct JsSet<T> {
    items: Vec<T>,
    index: std::collections::HashMap<T, usize>,
}

#[allow(dead_code)]
impl<T: std::hash::Hash + Eq + Clone> JsSet<T> {
    fn new() -> Self {
        JsSet { items: Vec::new(), index: std::collections::HashMap::new() }
    }
    fn len(&self) -> usize {
        self.items.len()
    }
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    fn contains<Q: ?Sized + std::hash::Hash + Eq>(&self, value: &Q) -> bool
    where
        T: std::borrow::Borrow<Q>,
    {
        self.index.contains_key(value)
    }
    /// `set.add(value)`: true when the value was not there.
    fn insert(&mut self, value: T) -> bool {
        if self.index.contains_key(&value) {
            return false;
        }
        self.index.insert(value.clone(), self.items.len());
        self.items.push(value);
        true
    }
    fn remove<Q: ?Sized + std::hash::Hash + Eq>(&mut self, value: &Q) -> bool
    where
        T: std::borrow::Borrow<Q>,
    {
        let Some(i) = self.index.remove(value) else {
            return false;
        };
        self.items.remove(i);
        for (_, pos) in self.index.iter_mut() {
            if *pos > i {
                *pos -= 1;
            }
        }
        true
    }
    fn clear(&mut self) {
        self.items.clear();
        self.index.clear();
    }
    fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }
}

impl<T: std::hash::Hash + Eq + Clone> Default for JsSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: std::hash::Hash + Eq + Clone> PartialEq for JsSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

impl<T: std::hash::Hash + Eq + Clone> FromIterator<T> for JsSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = JsSet::new();
        for v in iter {
            set.insert(v);
        }
        set
    }
}

impl<T: std::hash::Hash + Eq + Clone, const N: usize> From<[T; N]> for JsSet<T> {
    fn from(items: [T; N]) -> Self {
        items.into_iter().collect()
    }
}

impl<T: std::hash::Hash + Eq + Clone> Extend<T> for JsSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for v in iter {
            self.insert(v);
        }
    }
}

impl<T: std::hash::Hash + Eq + Clone> IntoIterator for JsSet<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a, T: std::hash::Hash + Eq + Clone> IntoIterator for &'a JsSet<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

