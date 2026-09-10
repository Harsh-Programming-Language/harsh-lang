
// Bracket groups: every `(` or `[` group has an anchor -- the callee path
// before the bracket, or the bracket itself -- with its contents one unit
// past the anchor and its closer on its own line under it. The shapes here
// are the ones "Layout style" in docs/LANGUAGE.md prescribes and accepts.
fn compute(a: i32, b: i32) -> i32 {
    a + b
}

fn show(t: (i32, i32)) -> i32 {
    t.0 + t.1
}

fn main() {
    let v = vec![1, 2, 3];

    // The prescribed chain form: line-final `(`, the closure one unit past
    // the callee, its body one unit past the closure, `)` under the callee.
    let d: Vec<i32> =
            v.iter()
              .map(
                     |x| {
        x * 10
    })
              .collect();

    // The accepted compact form: construct on the `(` line, one-line body,
    // the closer ending the body line.
    let e: Vec<i32> =
            v.iter()
              .map(|x| {
        x * 10
    })
              .collect();

    // A bare `(` or `[` anchors on itself. Inside, Rust's commas.
    let t =
            (
                1,
                2
            );
    let xs =
            [
                1,
                2
            ];
    let ys =
            vec![
                1,
                2
            ];

    // A call with several arguments breaks by juxtaposition: one argument
    // per continuation line, no closer.
    let s =
            compute(3,
                4);

    // The wider accepted forms: the anchor mid-line.
    let u = compute(1 + 1, 
                    2);
    let zs = [
                     5,
                     6
                 ];
    let w = show(
                    (7, 8)
                );

    println!("{:?} {:?} {:?} {:?} {:?} {} {} {:?} {}", d, e, t, xs, ys, s, u, zs, w)
}
