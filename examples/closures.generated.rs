fn double(v: Vec<i32>) -> Vec<i32> {
    v.iter().map(|n| {
        n * 2
    })
          .collect()
}

fn inline(v: Vec<i32>) -> Vec<i32> {
    v.iter().map(|n| n * 2).collect()
}

fn apply2(f: impl Fn(i32) -> i32) -> i32 {
    f(3)
}

fn main() {
    println!("{:?}", double(vec! [1, 2, 3]));
    println!("{:?}", inline(vec! [1, 2, 3]));
    let r = apply2(|x| {
        x + 100
    });
    println!("{r}")
}
