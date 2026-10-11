#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Level {
    Low,
    High,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Level::Low => "low",
            Level::High => "high",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Thermostat {
    _target: i64,
    pub name: String,
    pub history: Vec<i64>,
}

impl Thermostat {
    pub const MAX: i64 = 30;

    pub fn new(name: String) -> Self {
        Self { _target: 20, name, history: Vec::new() }
    }

    pub fn target(&self) -> i64 {
        self._target
    }

    pub fn set_target(&mut self, value: i64) {
        self._target = value.min(Thermostat::MAX);
        self.history.push(self._target);
    }

    pub fn level(&self) -> Level {
        if self._target >= 25 { Level::High } else { Level::Low }
    }

    pub fn to_string(&self) -> String {
        format!("{}@{}", self.name, self._target)
    }

    pub fn from_default() -> Thermostat {
        Thermostat::new("default".to_string())
    }
}

fn main() {
    let mut t = Thermostat::new("hall".to_string());
    t.set_target(24);
    t.set_target(40);
    println!("{} {} {} {}", t.target(), t.level(), t.history.len(), Thermostat::MAX);
    println!("{} {} {}", t.to_string(), t.to_string().to_string(), Thermostat::from_default().name);
    let level = Level::High;
    println!("{} {level}", level == Level::High);
}
