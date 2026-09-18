fn greet(name: &str) -> String {
    format!("Hello, {name}")
}
fn greet2(name: &str, age: i32) -> String {
    format!("Hello, {name}, age {}", age)
}
fn nothing() -> i64 {
    let mut sum = 0;
    sum
}
fn total(xs: &[i64], ys: &[i64]) -> i64 {
    let mut sum = 0;
    for x in xs {
        sum += x
    }
    for y in ys {
        sum += y
    }
    sum
}
fn apply(g: fn(i32) -> i32, v: i32) -> i32 {
    g(v)
}
fn commas(a: i32, b: i32) -> i32 {
    a + b
}
fn generic<T: std::fmt::Debug>(label: &str, item: T) -> String {
    format!("{label}: {item:?}")
}
fn unit_first(x: i32) -> i32 {
    x * 2
}
fn main() {
    println!("{}", greet("Ben"));
    println!("{}", greet2("Ben", 40));
    println!("{}", nothing());
    println!("{}", total(&[1, 2, 3], &[10, 20]));
    println!("{}", apply(| n | n + 1, 41));
    println!("{}", commas(2, 3));
    println!("{}", generic("vals", vec![1, 2]));
    println!("{}", unit_first(21))
}
