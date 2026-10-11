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
