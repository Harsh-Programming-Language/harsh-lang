# The mark

```
#[Ha<rs>.h]
```

Eleven characters. Five readings, none of which contradicts the others.

## What it says

### A word

- Read straight through, it spells **Hars.h** — the language's name, with a dot before the final letter.
- The dot is what turns a word into something that looks like a filename, and without it the mark is just a name in brackets.

### An attribute

- `#[…]` is Rust's attribute syntax, and an attribute is metadata annotating the thing below it.
- So the mark reads as *this code is Harsh*, which is the one sentence a logo on a source file would want to say.

### A generic

- `Ha<rs>` parses as a generic instantiation before the eye notices it spells a word, because `<rs>` sits exactly where a type parameter goes.
- This one was not designed. It arrived because the extension happens to be two letters and Rust happens to bracket its type parameters.

### A C header

- `hars.h` reads as a header file, and a header is the thing you drop into any project and it works.
- That is close to Harsh's actual claim — every Rust crate works unchanged, because Harsh is a layout transformation rather than a new language.

### The file extension

- The mark contains `.hrs`, split across the word.

```
#[Ha<rs>.h]
     │  │
     │  └── .h
     └───── rs
```

- Read the letters in order and it is `hars.h`, which is familiar. Read `.h` and `rs` as one unit and it is `.hrs`, which is the actual extension, hiding inside a spelling that looks like C.
- This is why the colour is structural rather than decorative. The orange is not only marking Rust; it marks the half of `.hrs` that was separated from the dot to make the word work, and it is what lets a reader find `rs` again once the letters have been rearranged.

## How it is set

### Typeface

- Monospace throughout, in the same stack the code blocks use, because the conceit is that the mark looks like something you could paste into a file.
- A serif was tried and rejected. The punctuation carries the joke, so a serif keeps the reference — but it shifts the register from *this is code* to *this is a book about code*.
- Roman, with one exception below. Setting the whole mark in italic trades the joke for decoration, since code is never italic.

### The lean

- The final `h` is obliqued by 11 degrees, so it leans against the closing bracket while `Ha<rs>.` stands up straight.
- It is an **oblique, not an italic** — the same glyph skewed, not a different face.

```css
.lg-i { display:inline-block; transform:skewX(-11deg); transform-origin:50% 70% }
```

- `font-style: italic` is wrong here and looks it. Most monospace faces have no italic, so the browser substitutes a serif and the `h` leaves the family; the skew never leaves it.
- The origin sits low so the letter pivots near its baseline rather than about its middle, which would make it drift sideways instead of leaning.
- The lean only works on the **final** character, because a lean is a posture and a posture needs something to lean on. Anywhere else in the string it reads as a mistake.

### Colour

| Part | Role | Colour |
|---|---|---|
| `#[` `]` | attribute punctuation, pushed back so the word reads first | `#5b6d79` grey |
| `Ha` `h` | the word | `#d7e1e8` foreground |
| `<` `>` `.` | the characters doing structural work rather than spelling | `#6fb3d2` blue |
| `rs` | Rust, and the displaced half of `.hrs` | `#e08b5a` orange |

- The grey is what makes the word stand out; without it the brackets compete with the letters they contain.
- The orange is the colour anyone who has read Rust code recognises, so the reference lands before the explanation does.

### Small sizes

- The full mark holds at display sizes and collapses below roughly 20px, where `<rs>` turns into a smudge.
- Use `#[H]`, or the bracket pair alone, wherever the mark has no room — a favicon, a tab strip, a badge.

## Reserved

- The mark is not licensed with the code. See `docs/TRADEMARK.md`.
- Anyone may fork Harsh; a fork must be called something else and must not use this mark.
