#[derive(Debug, Clone, PartialEq)]
struct User {
    pub name: String,
    pub age: i64,
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

fn adults(list: &[User]) -> Vec<User> {
    list.iter().filter(|u| u.age >= 18).cloned().collect::<Vec<_>>()
}

fn oldest(list: &[User]) -> Option<User> {
    let mut best: Option<User> = None;
    for u in list {
        if best.as_ref().is_none_or(|best| u.age > best.age) {
            best = Some(u.clone());
        }
    }
    best
}

fn main() {
    let users = vec![User { name: "Ann".to_string(), age: 31, email: Some("ann@example.com".to_string()) }, User { name: "Bob".to_string(), age: 17, email: None }, User { name: "Cid".to_string(), age: 45, email: None }];
    let grown = adults(&users);
    println!("{} {}", grown.len(), grown.iter().map(|u| u.name.clone()).collect::<Vec<_>>().join(", "));
    let top = oldest(&users);
    if let Some(top) = &top {
        println!("{} is the oldest ({})", top.name, top.age);
        println!("{}", top.email.clone().unwrap_or("no email".to_string()));
    }
    let copy = User { age: 32, ..users[0].clone() };
    println!("{} {} {}", copy.name, copy.age, users[0].age);
    let point = Point { x: 1, y: 2 };
    let moved = Point { x: point.x + 1, y: point.y };
    println!("{} {} {}", moved.x, moved.y, point.x);
    let obj_ = &users[1];
    let name = obj_.name.clone();
    let age = obj_.age;
    println!("{name} {age}");
}
