# 19. Patterns and matching

You have been writing patterns since chapter 2: `Ok n`, `Some max`, `(key, value)`, `Point { x, y }`. A *pattern* is a shape that a value is tested against and, when it fits, taken apart into names. This chapter collects every place a pattern can appear and every form one can take, so that the next time a value has a shape you can write the shape directly instead of reaching for it with methods. Harsh's contribution is the one you know: a variant with fields is matched by juxtaposition, `Coin.Quarter state`, the way it is constructed.

## 19.1 Where patterns appear

@@ places

Six places. `match` arms, where every case must be covered. `if let`, for one case, with `else if` and `else if let` chaining conditions and patterns freely — the example mixes a plain boolean between two pattern tests. `while let`, looping while a pattern keeps matching — popping a stack until it is empty. `for`, whose loop variable is a pattern: `(index, value)` takes apart the pairs `enumerate$` yields. `let`, whose left side is a pattern: `let (a, b, c) = …` is three bindings at once, and the plain `let x = 5` you have written a thousand times is a pattern too, one that matches anything. And function parameters, which are patterns as well: `&(x, y): &(i32, i32)` takes a reference to a tuple apart in the signature. The nested `fn` is legal Rust — an item inside a function, visible only there.

## 19.2 Refutability

A pattern that can fail to match is *refutable*; one that always matches is *irrefutable*. `Some x` is refutable, `x` and `(a, b)` are not. `let`, `for` and function parameters need an irrefutable pattern, because they have nothing to do when it fails; `if let`, `while let` and `match` arms accept a refutable one, because failing is what their `else`, their end, and their next arm are for:

@@ refutable !error

`let Some x = value` has nowhere to go when `value` is `None`, and the help offers the fix from chapter 6, `let … else`. The other direction is a warning rather than an error: `if let x = 5` is legal and pointless, and the compiler says so.

## 19.3 The forms

### Literals, names, ranges, alternatives

@@ literals

A literal matches itself. A name matches anything and binds it — and inside an arm it is a *new* variable that shadows any outer one, which is the trap in the second `match`: `Some y` does not compare against the outer `y`, it binds a fresh `y` to `5`, and the message says so. (The guard in 19.4 is how to compare against an outer variable.) `|` gives alternatives; `..=` matches an inclusive range of numbers or characters — ranges are the one pattern form that is not just a literal or a structure, and they are allowed only for those two types, where the compiler can check that a set of ranges covers everything.

### Destructuring

@@ destructure

A struct pattern is a field list like any other, marked with `\`: `Point\ x: a, y: b` binds the fields to new names — a rename keeps the colon, since here the field is not being given a value but matched — `Point\ x, y` is the shorthand for the same names, and a field may hold a literal to test it: `Point\ x, y: 0` matches only points on the x axis and binds `x`. Enum variants are matched by the shape that built them: `Message.Quit` bare, `Message.Move { x, y }` with the record's fields, `Message.Write text` with one juxtaposed name, and `Message.ChangeColor (Color.Hsv h s v)` with the inner variant's pattern *isolated in parentheses*, because it is one argument and it has structure. The isolating parentheses are Harsh's argument rule, applied to a pattern — the same rule as `Some (i + 1)` on the constructing side. And patterns nest to any depth: the last `let` takes a tuple of a tuple and a struct apart in one line.

### Ignoring

@@ ignoring

`_` matches anything and binds nothing. In a parameter it is a value the function ignores (useful when a trait signature requires it); nested, it tests a shape without taking it apart — `(Some _, Some _)` is "both set" without caring what to; in a tuple it skips positions. `_` and a name starting with `_` differ in one way that matters: `_x` *binds* (and only silences the unused-variable warning), so `if let Some _s = s` would move the `String` out of `s`, while `if let Some _ = s` does not, and `s` is still printable after. `..` ignores *all remaining* parts: `Point { x, .. }` for a struct, `(first, .., last)` for a tuple, and it must be unambiguous — `(.., second, ..)` is an error, since the compiler cannot tell which position `second` means.

## 19.4 Guards and bindings

@@ guards_bindings

A *match guard* is an `if` after the pattern: the arm matches only if the pattern fits *and* the condition holds. `Some x if x % 2 == 0` tests the bound value; `Some n if n == y` compares against the outer `y` — the answer to the shadowing trap, since a guard is an expression and sees the enclosing scope. A guard applies to the whole of an or-pattern, `4 | 5 | 6 if y`, not just the last alternative. Guards are not counted for exhaustiveness — the compiler cannot see through an arbitrary condition — so a `match` whose arms all have guards still needs a catch-all.

`@` binds a name to a value *while* testing it: `id_variable @ 3..=7` matches ids from 3 to 7 and gives the arm the actual id, where `3..=7` alone would test without binding and `id` alone would bind without testing. It is the form for "I want to know it is in this range, and I want the value".

## 19.5 What you have

Patterns appear in `match`, `if let`, `while let`, `for`, `let` and parameters; `let`, `for` and parameters need irrefutable ones. The forms: literals, names (which shadow), `|`, `..=` ranges, tuple and struct and variant destructuring to any depth, `_` and `_name` and `..` to ignore, `if` guards, and `x @ pattern` to bind and test. Harsh writes a variant's payload by juxtaposition and isolates a nested pattern in parentheses, exactly as it writes the constructing expression.

Next: the advanced features — `unsafe`, the corners of traits and types, function pointers and returned closures, and macros — which most programs never need and every Rust programmer eventually meets.
