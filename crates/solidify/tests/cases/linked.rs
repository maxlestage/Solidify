#[derive(Debug, Clone, PartialEq)]
struct Node {
    pub value: i64,
    pub next: Option<Box<Node>>,
}

impl Node {
    pub fn new(value: i64) -> Self {
        Self { value, next: None }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct List {
    pub head: Option<Box<Node>>,
    pub size: i64,
}

impl List {
    pub fn new() -> Self {
        Self { head: None, size: 0 }
    }

    pub fn push(&mut self, value: i64) {
        let node = Node::new(value);
        if let Some(head) = self.head.as_deref_mut() {
            let mut current = head;
            while current.next.is_some() {
                current = current.next.as_mut().unwrap();
            }
            current.next = Some(Box::new(node));
        } else {
            self.head = Some(Box::new(node));
        }
        self.size += 1;
    }

    pub fn sum(&self) -> i64 {
        let mut total = 0;
        let mut current = &self.head;
        while current.is_some() {
            total += current.as_ref().unwrap().value;
            current = &current.as_ref().unwrap().next;
        }
        total
    }

    pub fn first(&self) -> Option<i64> {
        let Some(head) = self.head.as_deref() else {
            return None;
        };
        Some(head.value)
    }
}

fn main() {
    let mut list = List::new();
    list.push(3);
    list.push(4);
    list.push(5);
    println!("{} {} {}", list.size, list.sum(), list.first().as_ref().map_or("null".to_string(), |v| v.to_string()));
    let empty = List::new();
    let f = empty.first();
    println!("{}", if let Some(f) = f { format!("first {f}") } else { "empty".to_string() });
}
