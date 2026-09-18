fn sub(a: i32) -> impl Fn(i32) -> i32 {
    move |b| a - b
}

fn main() {
    let u = (sub(3))(100);
    let v = (sub(100))(3);
    let x = (sub(100))(3);
    println!("u={u} v={v} x={x}")
}
