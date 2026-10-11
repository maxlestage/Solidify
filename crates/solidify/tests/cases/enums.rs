#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Color {
    Red,
    Green,
    Blue,
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Color::Red => "0",
            Color::Green => "1",
            Color::Blue => "2",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Direction::Up => "up",
            Direction::Down => "down",
            Direction::Left => "left",
            Direction::Right => "right",
        })
    }
}

fn hex(c: Color) -> String {
    match c {
        Color::Red => {
            return "#f00".to_string();
        }
        Color::Green => {
            return "#0f0".to_string();
        }
        Color::Blue => {
            return "#00f".to_string();
        }
    }
}

fn step(d: Direction) -> i64 {
    match d {
        Direction::Up | Direction::Right => {
            return 1;
        }
        _ => {
            return -1;
        }
    }
}

fn classify(n: i64) -> String {
    match n {
        0 => {
            return "zero".to_string();
        }
        1 | 2 => {
            return "small".to_string();
        }
        _ => {
            return "big".to_string();
        }
    }
}

fn main() {
    println!("{} {} {} {} {}", hex(Color::Green), step(Direction::Left), step(Direction::Up), classify(2), classify(7));
    let mut dir = Direction::Down;
    if dir == Direction::Down {
        dir = Direction::Up;
    }
    println!("{dir} {}", Color::Blue);
    let colors = vec![Color::Red, Color::Blue];
    println!("{} {}", colors.len(), colors.contains(&Color::Blue));
}
