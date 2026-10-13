fn main() {
    let v = vec![1, 2, 3];
    let s = format!("{} items, first {:?}", v.len(), v.first());
    assert_eq!(v.iter().sum::<i32>(), 6, "sum of {:?}", v);
    let big = matches!(v.last(), Some(n) if *n > 2);
    println!("{s} {big}");
}
