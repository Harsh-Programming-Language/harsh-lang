//! The source map through a macro expansion: an error inside expanded code
//! points at the definition's line, and a note names the call that produced
//! it. Driven through the real binaries, as a build would.

use std::fs;
use std::process::Command;

#[test]
fn an_error_inside_an_expansion_names_the_definition_and_the_call() {
    let dir = std::env::temp_dir().join(format!("harsh-remap-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let hrs = dir.join("m.hrs");
    fs::write(
        &hrs,
        "macro_rules~ bad\n    (($e:expr)) => do:\n        $e +\n\nfn main$:\n    let n = bad~ 1\n    println! \"{n}\"\n",
    )
    .unwrap();
    let rs = dir.join("m.rs");
    let map = dir.join("m.map.json");
    let t = Command::new(env!("CARGO_BIN_EXE_hrs")).arg(&hrs).arg("-o").arg(&rs).arg("--map").arg(&map).output().unwrap();
    assert!(t.status.success(), "{}", String::from_utf8_lossy(&t.stderr));

    // rustc's JSON diagnostics, wrapped as cargo emits them.
    let c = Command::new("rustc")
        .args(["--edition", "2021", "--error-format=json", "-A", "warnings", "--crate-type", "bin", "-o"])
        .arg(dir.join("m.bin"))
        .arg(&rs)
        .output()
        .unwrap();
    assert!(!c.status.success(), "the expansion should not compile");
    let wrapped: String = String::from_utf8_lossy(&c.stderr)
        .lines()
        .filter(|l| l.starts_with('{'))
        .map(|l| format!("{{\"reason\":\"compiler-message\",\"message\":{l}}}\n"))
        .collect();

    let r = Command::new(env!("CARGO_BIN_EXE_hrs-remap"))
        .arg("--map")
        .arg(&map)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    r.stdin.as_ref().unwrap().write_all(wrapped.as_bytes()).unwrap();
    let out = r.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    let _ = fs::remove_dir_all(&dir);

    // the transcriber's line, and the call's
    assert!(text.contains("m.hrs:3:13"), "{text}");
    assert!(text.contains("$e +"), "{text}");
    assert!(text.contains("in the expansion of `bad~` called at"), "{text}");
    assert!(text.contains("m.hrs:6:13"), "{text}");
    assert!(text.contains("let n = bad~ 1"), "{text}");
}

/// An error inside a hole, and one after the body and after a `macro_rules!`
/// zone, point at the `.hrs` line and column (2026-09-23). Until then a hole
/// carried no map entries, `dslzone::restore` moved the text under the
/// entries after it, a placeholder that outgrew its body's first line pushed
/// every source offset after it, and `rawzone::restore` had never moved the
/// map at all -- so any file with a `macro_rules!` mapped wrong after it.
#[test]
fn an_error_inside_a_hole_points_at_the_hole() {
    let dir = std::env::temp_dir().join(format!("harsh-remap-hole-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let hrs = dir.join("h.hrs");
    fs::write(
        &hrs,
        "macro_rules! show {\n    ($e:expr) => { println!(\"{}\", $e) };\n}\n\nfn main$:\n    let count = 3\n    show! {\n        @: count + \"x\" :@\n    }\n    let bad: u8 = \"later\"\n",
    )
    .unwrap();
    let rs = dir.join("h.rs");
    let map = dir.join("h.map.json");
    let t = Command::new(env!("CARGO_BIN_EXE_hrs")).arg(&hrs).arg("-o").arg(&rs).arg("--map").arg(&map).output().unwrap();
    assert!(t.status.success(), "{}", String::from_utf8_lossy(&t.stderr));
    let c = Command::new("rustc")
        .args(["--edition", "2021", "--error-format=json", "-A", "warnings", "--crate-type", "bin", "-o"])
        .arg(dir.join("h.bin"))
        .arg(&rs)
        .output()
        .unwrap();
    assert!(!c.status.success(), "the file should not compile");
    let wrapped: String = String::from_utf8_lossy(&c.stderr)
        .lines()
        .filter(|l| l.starts_with('{'))
        .map(|l| format!("{{\"reason\":\"compiler-message\",\"message\":{l}}}\n"))
        .collect();
    let r = Command::new(env!("CARGO_BIN_EXE_hrs-remap"))
        .arg("--map")
        .arg(&map)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    r.stdin.as_ref().unwrap().write_all(wrapped.as_bytes()).unwrap();
    let out = r.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    let _ = fs::remove_dir_all(&dir);
    // the `+` of the hole, and the `"later"` after the body
    assert!(text.contains("h.hrs:8:18"), "{text}");
    assert!(text.contains("h.hrs:10:19"), "{text}");
}
