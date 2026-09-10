; Harsh highlighting. The token set is Rust's; only the operators differ.

(line_comment) @comment
(block_comment) @comment

(string) @string
(char) @string
(number) @number
(lifetime) @label

(macro_name) @function.special
(metavariable) @variable.special
(apply_nothing) @punctuation.special

(keyword) @keyword
(operator) @operator
(punctuation) @punctuation.delimiter
(attribute) @attribute

; The block-opening `:` and `=>` read as structure rather than punctuation.
(header opener: _ @punctuation.special)

; A name directly before an argument run is being called.
(header (keyword) @keyword.function
  (#match? @keyword.function "^(fn)$"))

((identifier) @type
  (#match? @type "^[A-Z]"))

((identifier) @constant
  (#match? @constant "^[A-Z][A-Z0-9_]+$"))

; A block opened inside a paren -- an isolated closure body -- reads as
; structure, like any other block opener.
(paren_block opener: _ @punctuation.special)
