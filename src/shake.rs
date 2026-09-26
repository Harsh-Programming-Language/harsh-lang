//! Item-level shaking of `hrs_std` for `hrs export` (the user, 2026-09-25;
//! `docs/dev/PACKAGING.md` section 4): the exported crate gets only the items
//! of `hrs_std` its code reaches, closed over what each kept item uses.
//!
//! Items are read with the transpiler's own Rust lexer: every top-level item
//! with its attributes and doc comments. An item is kept when
//! - it defines a name that is reached;
//! - it is an `impl` whose type is reached (it may give that type an operator,
//!   `Display`, a trait the code uses without naming);
//! - it invokes a macro (`numbers!(…)`, generating `impl`s) and names, or its
//!   macro's definition names, something reached;
//! - it is a `use` or a `mod` declaration -- kept, but a `use` list is pruned
//!   of names defined here and not kept.
//! A kept item's identifiers are reached in turn, to a fixed point.
//! Conservative by construction: reached means *named*, so an item named in a
//! kept item's body stays even if that body path is never taken.

use crate::lex::{lex_rust, Tk, Token};
use std::collections::HashSet;

#[derive(Debug)]
struct Item {
    lo: usize,
    hi: usize,
    defines: Vec<String>,
    impl_type: Option<String>,
    /// The trait an `impl … for …` implements.
    impl_trait: Option<String>,
    invokes: Option<String>,
    is_use: bool,
    is_mod: bool,
    idents: HashSet<String>,
}

/// The top-level items of a Rust file.
fn items(text: &str) -> Vec<Item> {
    let Ok(toks) = lex_rust(text) else { return Vec::new() };
    let t: Vec<&Token> = toks.iter().filter(|x| !x.is_comment() || x.text.starts_with("///") || x.text.starts_with("/**")).collect();
    let mut out = Vec::new();
    let mut k = 0;
    while k < t.len() {
        let start = k;
        // attributes and doc comments belong to the item after them
        let mut i = k;
        loop {
            if i < t.len() && t[i].is_comment() {
                i += 1;
            } else if i + 1 < t.len() && t[i].text == "#" && t[i + 1].text == "[" {
                let mut d = 0i32;
                i += 1;
                while i < t.len() {
                    match t[i].kind {
                        Tk::Open(_) => d += 1,
                        Tk::Close(_) => {
                            d -= 1;
                            if d == 0 {
                                i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
            } else {
                break;
            }
        }
        let head = i;
        if head >= t.len() {
            break;
        }
        let is_use = (0..3).any(|n| t.get(head + n).map_or(false, |x| x.text == "use")) && t[head..].iter().take(4).any(|x| x.text == "use");
        // the item's end: `;` at depth 0, or the `}` that closes a depth-0
        // `{` (and a `;` right after it)
        let mut d = 0i32;
        let mut j = head;
        let mut end = t.len() - 1;
        while j < t.len() {
            match t[j].kind {
                Tk::Open(_) => d += 1,
                Tk::Close(c) => {
                    d -= 1;
                    if d == 0 && c == '}' && !is_use {
                        end = if t.get(j + 1).map_or(false, |x| x.text == ";") { j + 1 } else { j };
                        break;
                    }
                }
                Tk::Semi if d == 0 => {
                    end = j;
                    break;
                }
                _ => {}
            }
            j += 1;
        }
        let body = &t[head..=end];
        let word = |w: &str| body.iter().position(|x| x.text == w && x.kind == Tk::Ident);
        let mut defines = Vec::new();
        for kw in ["fn", "struct", "enum", "trait", "type", "const", "static", "mod", "union"] {
            if let Some(p) = word(kw) {
                // only a keyword before any `{` or `(` names this item
                let before_body = body[..p].iter().all(|x| !matches!(x.kind, Tk::Open('{') | Tk::Open('(')));
                if before_body {
                    if let Some(n) = body.get(p + 1).filter(|x| x.kind == Tk::Ident) {
                        defines.push(n.text.clone());
                    }
                }
            }
        }
        if body.len() > 2 && body[0].text == "macro_rules" && body[1].text == "!" {
            defines.push(body[2].text.clone());
        }
        let mut impl_type = None;
        let mut impl_trait = None;
        if body.first().map_or(false, |x| x.text == "impl") || body.iter().take(3).any(|x| x.text == "impl") {
            let at_for = body.iter().position(|x| x.text == "for" && x.kind == Tk::Ident);
            let from = match at_for {
                Some(p) => p + 1,
                None => {
                    // skip `impl<…>`
                    let mut q = 1;
                    if body.get(1).map_or(false, |x| x.text == "<") {
                        let mut a = 0;
                        while q < body.len() {
                            if body[q].text == "<" {
                                a += 1;
                            } else if body[q].text == ">" {
                                a -= 1;
                                if a == 0 {
                                    q += 1;
                                    break;
                                }
                            }
                            q += 1;
                        }
                    }
                    q
                }
            };
            impl_type = body[from..].iter().find(|x| x.kind == Tk::Ident && x.text != "dyn").map(|x| x.text.clone());
            if let Some(p) = at_for {
                // the trait: the last path segment before `<` or `for`
                let mut q = 1;
                if body.get(1).map_or(false, |x| x.text == "<") {
                    let mut a = 0;
                    while q < p {
                        if body[q].text == "<" {
                            a += 1;
                        } else if body[q].text == ">" {
                            a -= 1;
                            if a == 0 {
                                q += 1;
                                break;
                            }
                        }
                        q += 1;
                    }
                }
                impl_trait = body[q..p].iter().take_while(|x| x.text != "<").filter(|x| x.kind == Tk::Ident).last().map(|x| x.text.clone());
            }
        }
        let invokes = match (body.first(), body.get(1)) {
            (Some(n), Some(b)) if n.kind == Tk::Ident && b.text == "!" && n.text != "macro_rules" => Some(n.text.clone()),
            _ => None,
        };
        let is_mod = body.first().map_or(false, |x| x.text == "mod") || (body.first().map_or(false, |x| x.text == "pub") && body.get(1).map_or(false, |x| x.text == "mod"));
        let idents = body.iter().filter(|x| x.kind == Tk::Ident).map(|x| x.text.clone()).collect();
        out.push(Item {
            lo: t[start].span.lo as usize,
            hi: t[end].span.hi as usize,
            defines,
            impl_type,
            impl_trait,
            invokes,
            is_use,
            is_mod,
            idents,
        });
        k = end + 1;
    }
    out
}

/// `files` (name, text) shaken to what `roots` reaches. Returns the files'
/// new texts.
pub fn shake(files: &[(String, String)], roots: &HashSet<String>) -> Vec<(String, String)> {
    let all: Vec<(usize, Item)> = files.iter().enumerate().flat_map(|(f, (_, t))| items(t).into_iter().map(move |i| (f, i))).collect();
    let defined: HashSet<String> = all.iter().flat_map(|(_, i)| i.defines.clone()).collect();
    let macro_defs: std::collections::HashMap<String, &HashSet<String>> =
        all.iter().filter(|(_, i)| !i.defines.is_empty() && files[0].1.len() > 0).flat_map(|(_, i)| i.defines.iter().map(move |d| (d.clone(), &i.idents))).collect();
    let mut reached: HashSet<String> = roots.clone();
    let mut kept = vec![false; all.len()];
    loop {
        let mut changed = false;
        for (n, (_, it)) in all.iter().enumerate() {
            if kept[n] {
                continue;
            }
            let keep = it.is_use
                || it.is_mod
                || it.defines.iter().any(|d| reached.contains(d))
                || it.impl_type.as_ref().map_or(false, |ty| reached.contains(ty))
                // a blanket impl (`Operand for &'a X`), or one for a type not
                // defined here (`Operand for f64`): kept when its trait is
                || (it.impl_trait.as_ref().map_or(false, |tr| reached.contains(tr))
                    && it.impl_type.as_ref().map_or(true, |ty| !defined.contains(ty)))
                || it.invokes.as_ref().map_or(false, |m| {
                    it.idents.iter().any(|x| x != m && reached.contains(x))
                        || macro_defs.get(m).map_or(false, |def| def.iter().any(|x| x != m && reached.contains(x)))
                });
            if keep {
                kept[n] = true;
                changed = true;
                // a `use` or `mod` line keeps, but does not reach
                if !it.is_use && !it.is_mod {
                    for x in &it.idents {
                        reached.insert(x.clone());
                    }
                    if let Some(m) = &it.invokes {
                        reached.insert(m.clone());
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    let kept_names: HashSet<String> =
        all.iter().zip(&kept).filter(|(_, k)| **k).flat_map(|((_, i), _)| i.defines.clone()).collect();
    files
        .iter()
        .enumerate()
        .map(|(f, (name, text))| {
            let mut out = text.clone();
            let mine: Vec<(&Item, bool)> = all.iter().zip(&kept).filter(|((g, _), _)| *g == f).map(|((_, i), k)| (i, *k)).collect();
            for (it, keep) in mine.into_iter().rev() {
                if !keep {
                    out.replace_range(it.lo..it.hi, "");
                } else if it.is_use {
                    let pruned = prune_use(&text[it.lo..it.hi], &defined, &kept_names);
                    out.replace_range(it.lo..it.hi, &pruned);
                }
            }
            (name.clone(), out)
        })
        .collect()
}

/// A `use` line with the names defined here but not kept taken out of it;
/// empty when none is left.
fn prune_use(line: &str, defined: &HashSet<String>, kept: &HashSet<String>) -> String {
    let gone = |n: &str| defined.contains(n) && !kept.contains(n);
    let Some(open) = line.find('{') else {
        // `use a::b::Name;`
        let last = line.trim_end_matches(';').rsplit("::").next().unwrap_or("").trim().to_string();
        return if gone(&last) { String::new() } else { line.to_string() };
    };
    let Some(close) = line.rfind('}') else { return line.to_string() };
    let names: Vec<&str> = line[open + 1..close].split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let left: Vec<&str> = names.into_iter().filter(|n| !gone(n.split_whitespace().next().unwrap_or(n))).collect();
    if left.is_empty() {
        return String::new();
    }
    format!("{}{}{}", &line[..open + 1], left.join(", "), &line[close..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_not_reached_goes_and_what_is_stays() {
        let src = "use std::fmt;\npub struct A;\npub struct B;\nimpl fmt::Display for A {\n    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { helper(f) }\n}\nfn helper(f: &mut fmt::Formatter) -> fmt::Result { Ok(()) }\nimpl B { pub fn b() {} }\npub fn unused() {}\n";
        let roots: HashSet<String> = ["A".to_string()].into();
        // (Destructured: the converter misreads `[0].1` -- the self-host.)
        let shaken = shake(&[("lib.rs".into(), src.into())], &roots);
        let (_, out) = &shaken[0];
        assert!(out.contains("pub struct A;") && out.contains("impl fmt::Display for A") && out.contains("fn helper"), "{out}");
        assert!(!out.contains("pub struct B;") && !out.contains("impl B") && !out.contains("fn unused"), "{out}");
    }

    #[test]
    fn a_use_list_loses_what_went() {
        let defined: HashSet<String> = ["A", "B"].iter().map(|s| s.to_string()).collect();
        let kept: HashSet<String> = ["A"].iter().map(|s| s.to_string()).collect();
        assert_eq!(prune_use("pub use m::{A, B, Ext};", &defined, &kept), "pub use m::{A, Ext};");
        assert_eq!(prune_use("pub use m::B;", &defined, &kept), "");
    }
}
