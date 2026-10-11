#[derive(Debug, Clone, PartialEq)]
struct Tree {
    pub value: i64,
    pub left: Option<Box<Tree>>,
    pub right: Option<Box<Tree>>,
}

fn leaf(value: i64) -> Tree {
    Tree { value, left: None, right: None }
}

fn insert(mut root: Option<Box<Tree>>, value: i64) -> Tree {
    let Some(root) = root.as_deref_mut() else {
        return leaf(value);
    };
    if value < root.value {
        root.left = Some(Box::new(insert(root.left.clone(), value)));
    } else {
        root.right = Some(Box::new(insert(root.right.clone(), value)));
    }
    root.clone()
}

fn sum(root: Option<&Tree>) -> i64 {
    let Some(root) = root else {
        return 0;
    };
    root.value + sum(root.left.as_deref()) + sum(root.right.as_deref())
}

fn depth(root: Option<&Tree>) -> i64 {
    let Some(root) = root else {
        return 0;
    };
    1 + depth(root.left.as_deref()).max(depth(root.right.as_deref()))
}

fn find(root: Option<&Tree>, value: i64) -> Option<Box<Tree>> {
    let mut current = root.map(|v| Box::new(v.clone()));
    while current.is_some() && current.as_ref().unwrap().value != value {
        current = if value < current.as_ref().unwrap().value { current.as_ref().unwrap().left.clone() } else { current.as_ref().unwrap().right.clone() };
    }
    current
}

fn main() {
    let mut root: Option<Box<Tree>> = None;
    for v in vec![5, 3, 8, 1, 4, 9] {
        root = Some(Box::new(insert(root.clone(), v)));
    }
    println!("{} {}", sum(root.as_deref()), depth(root.as_deref()));
    let found = find(root.as_deref(), 4);
    println!("{} {}", found.map(|v| v.value).as_ref().map_or("undefined".to_string(), |v| v.to_string()), find(root.as_deref(), 7).as_ref().map(|v| v.value).unwrap_or(-1));
    let name: Option<String> = None;
    println!("{} {}", name.clone().unwrap_or("anonymous".to_string()), name.map(|v| v.chars().count() as i64).unwrap_or(0));
}
