use std::collections::HashMap;
use std::fmt;
const MAX_DEPTH: usize = 8;
static GREETING: &str = "tokenizing";
mod geometry {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Vec2 {
        pub x: f64,
        pub y: f64,
    }
    impl Vec2 {
        pub fn dot(self, o: Vec2) -> f64 {
            self.x * o.x + self.y * o.y
        }
    }
}
pub trait Shape {
    fn area(&self) -> f64;
    fn describe(&self) -> String {
        format!("shape with area {:.2}", self.area())
    }
}
pub struct Circle {
    r: f64,
}
impl Shape for Circle {
    fn area(&self) -> f64 {
        3.14159265358979 * self.r * self.r
    }
}
impl fmt::Display for Circle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Circle(r={})", self.r)
    }
}
#[derive(Debug)]
pub enum ParseErr {
    Empty,
    BadDigit(char),
}
fn parse_all<'a, I>(items: I) -> Result<Vec<i64>, ParseErr> where I: Iterator<Item = &'a str> {
    let mut out = Vec::<i64>:: new();
    for s in items {
        if s.is_empty() {
            return Err(ParseErr::Empty)
        }
        let mut n: i64 = 0;
        for c in s.chars() {
            match c.to_digit(10) {
                Some(d) => n = n * 10 + d as i64,
                None => return Err(ParseErr::BadDigit(c)),
            }
        }
        out.push(n)
    }
    Ok(out)
}
fn tally<T: AsRef<str>>(words: &[T]) -> HashMap<String, usize> {
    let mut m = HashMap::<String, usize>:: new();
    for w in words {
        let k = w.as_ref().to_lowercase();
        let e = m.entry(k).or_insert(0);
        * e += 1
    }
    m
}
fn main() {
    let c = Circle { r: 2.0 };
    println!("{} -> {}", c, c.describe());
    let v = geometry::Vec2 { x: 1.0, y: 2.0 };
    println!("dot = {}", v.dot(v));
    println!("{} up to depth {}", GREETING, MAX_DEPTH);
    match parse_all(["12", "34", "5"].into_iter()) {
        Ok(ns) => {
            let total: i64 = ns.iter().sum();
            println!("parsed {:?} total {}", ns, total)
        },
        Err(e) => println!("error: {:?}", e),
    }
    match parse_all(["7", "x9"].into_iter()) {
        Ok(ns) => println!("unexpected {:?}", ns),
        Err(e) => println!("rejected as {:?}", e),
    }
    let squares: Vec<i64> = (1..=5).map(| n | n * n).filter(| n | n % 2 == 1).collect();
    println!("odd squares {:?}", squares);
    let words = ["Alpha", "beta", "ALPHA", "Beta", "gamma"];
    let mut counts: Vec<(String, usize)> = tally(&words).into_iter().collect();
    counts.sort();
    for (w, n) in &counts {
        println!("  {w:>6} {n}")
    }
    let mut stack = vec![1, 2, 3];
    while let Some(top) = stack.pop() {
        print!("{top} ")
    }
    println!();
    if let Some(first) = words.first() {
        println!("first = {first}")
    }
}
