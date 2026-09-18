fn double(n: i32) -> i32 {
    n * 2
}

fn show(n: i32) -> String {
    format!("{n}")
}

fn main() {
    let a = double(21);
    let b = show(double(3));
    let c = double(5);
    let d = show(double(5));
    println!("{a} {b} {c} {d}")
}
