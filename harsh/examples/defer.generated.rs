fn sub3(a: i32, b: i32, c: i32) -> i32 {
    a - b - c
}

fn sub2(a: i32, b: i32) -> i32 {
    a - b
}

fn apply1(f: impl Fn(i32) -> i32, v: i32) -> i32 {
    f(v)
}

fn main() {
    // supply one from the left, defer the rest
    let one = move |__hrs1| sub2(10, __hrs1);
    let two = move |__hrs1, __hrs2| sub3(100, __hrs1, __hrs2);
    // supply from the right
    let last = move |__hrs1, __hrs2| sub3(__hrs1, __hrs2, 1);
    // the middle hole: left and right on one function
    let mid = move |__hrs1| sub3(100, __hrs1, 1);
    // compose: a partial of a partial, named or anonymous
    let two_20 = move |__hrs1| two(20, __hrs1);
    let t = move |__hrs1| (move |__hrs1, __hrs2| sub3(100, __hrs1, __hrs2))(20, __hrs1);
    // a partial is a value: passed, then applied
    let v = (move |__hrs1| sub2(30, __hrs1))(7);
    let w = apply1(move |__hrs1| sub2(9, __hrs1), 4);
    // every parameter supplied: a plain call
    let c = sub3(100, 20, 5);
    println!("{} {} {} {} {} {} {} {} {}", one(3), two(20, 5), last(100, 20), mid(20), two_20(5), t(5), v, w, c)
}
