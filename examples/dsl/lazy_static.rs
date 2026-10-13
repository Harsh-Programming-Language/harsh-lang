use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref NAMES: HashMap<u32, &'static str> = {
        let mut m = HashMap::new();
        m.insert(0, "zero");
        m.insert(1, "one");
        m
    };
    static ref COUNT: usize = NAMES.len() * 2;
}

fn main() {
    println!("{} {}", NAMES[&1], *COUNT);
}
