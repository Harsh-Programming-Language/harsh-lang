// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Lexer for hrs.
//!
//! Produces a flat token stream with byte spans into the original source. The
//! lexer knows nothing about blocks; it only records, for each token, whether it
//! is the first on a physical line and what that line's indentation is. The
//! layout pass consumes that.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub lo: u32,
    pub hi: u32,
}

impl Span {
    pub fn new(lo: usize, hi: usize) -> Self {
        Span { lo: lo as u32, hi: hi as u32 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tk {
    Ident,
    Lifetime,
    Int,
    Float,
    Str,
    Char,
    /// `.` used as a path separator (becomes `::`)
    Dot,
    /// `.0`, `.1` — tuple index, stays a Rust `.`
    TupleIdx,
    /// `..` and `..=`
    DotDot,
    /// `<-` — field access / method call (becomes `.`)
    LArrow,
    /// `<|` — apply the function on the left to the argument on the right
    PipeBack,
    /// `|>` — apply the function on the right to the value on the left
    PipeFwd,
    Colon,
    /// `::` — Rust's path separator, still legal in Harsh
    PathSep,
    Semi,
    Comma,
    FatArrow,
    Eq,
    Lt,
    Gt,
    Hash,
    /// `\` -- opens a struct's field list: a literal `Point\ x = 1`, an inline
    /// declaration `struct Point\ x: f64`, a pattern `Point\ x, ..`.
    Backslash,
    Open(char),
    Close(char),
    /// Any other punctuation, kept verbatim.
    Punct,
    LineComment,
    BlockComment,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Tk,
    pub span: Span,
    pub text: String,
    /// Indentation (in columns) of the physical line this token starts, if it is
    /// the first token on that line.
    pub line_start: Option<usize>,
    pub line: usize,
    /// Inserted by a rewrite rather than read from the source. The emitter must
    /// not copy source whitespace around these, since their spans no longer
    /// describe where they sit in the output.
    pub synthetic: bool,
    /// One of the two parens that `f$` becomes. Synthetic, but it stands
    /// exactly where the `$` stood (its span is the `$`'s), so the whitespace
    /// after it is the source's and is copied like any other token's.
    pub dollar: bool,
}

impl Token {
    pub fn is(&self, k: Tk) -> bool {
        self.kind == k
    }
    pub fn is_kw(&self, kw: &str) -> bool {
        self.kind == Tk::Ident && self.text == kw
    }
    pub fn is_comment(&self) -> bool {
        matches!(self.kind, Tk::LineComment | Tk::BlockComment)
    }
}

/// Which language the lexer is reading.
///
/// The two differ in exactly one place: what `.` means. In Harsh it is the path
/// separator; in Rust it is field access. Everything else -- literals, comments,
/// operators, indentation tracking -- is shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Harsh,
    Rust,
}

pub struct Lexer<'a> {
    src: &'a [u8],
    chars: Vec<(usize, char)>,
    i: usize,
    line: usize,
    toks: Vec<Token>,
    at_line_start: bool,
    pending_indent: usize,
    mode: Mode,
}

#[derive(Debug)]
pub struct LexError {
    pub msg: String,
    pub span: Span,
}

/// Lex Harsh source.
pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(src, Mode::Harsh).run()
}

/// Lex Rust source, for the Rust -> Harsh direction.
pub fn lex_rust(src: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(src, Mode::Rust).run()
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str, mode: Mode) -> Self {
        Lexer {
            src: src.as_bytes(),
            chars: src.char_indices().collect(),
            i: 0,
            line: 1,
            toks: Vec::new(),
            at_line_start: true,
            pending_indent: 0,
            mode,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.i).map(|&(_, c)| c)
    }
    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.i + n).map(|&(_, c)| c)
    }
    fn off(&self) -> usize {
        self.chars.get(self.i).map(|&(b, _)| b).unwrap_or(self.src.len())
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.i += 1;
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn push(&mut self, kind: Tk, lo: usize) {
        let hi = self.off();
        let text = String::from_utf8_lossy(&self.src[lo..hi]).into_owned();
        let ls = if self.at_line_start { Some(self.pending_indent) } else { None };
        self.at_line_start = false;
        self.toks.push(Token {
            kind,
            span: Span::new(lo, hi),
            text,
            line_start: ls,
            line: self.line,
            synthetic: false,
            dollar: false,
        });
    }

    fn run(mut self) -> Result<Vec<Token>, LexError> {
        loop {
            // Leading whitespace: track indentation, but only meaningfully at line start.
            if self.at_line_start {
                let mut col = 0usize;
                loop {
                    match self.peek() {
                        Some(' ') => {
                            col += 1;
                            self.i += 1;
                        }
                        Some('\t') => {
                            // Tabs advance to the next multiple of 4. Mixing tabs and
                            // spaces is legal but discouraged; the linter can flag it.
                            col = (col / 4 + 1) * 4;
                            self.i += 1;
                        }
                        _ => break,
                    }
                }
                self.pending_indent = col;
            }

            let c = match self.peek() {
                None => break,
                Some(c) => c,
            };

            if c == '\n' {
                self.bump();
                self.at_line_start = true;
                continue;
            }
            if c == '\r' || c == ' ' || c == '\t' {
                self.i += 1;
                continue;
            }

            let lo = self.off();

            // Comments
            if c == '/' && self.peek_at(1) == Some('/') {
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.i += 1;
                }
                self.push(Tk::LineComment, lo);
                continue;
            }
            if c == '/' && self.peek_at(1) == Some('*') {
                self.i += 2;
                let mut depth = 1usize;
                while depth > 0 {
                    match self.peek() {
                        None => {
                            return Err(LexError {
                                msg: "unterminated block comment".into(),
                                span: Span::new(lo, self.off()),
                            })
                        }
                        Some('/') if self.peek_at(1) == Some('*') => {
                            self.bump();
                            self.bump();
                            depth += 1;
                        }
                        Some('*') if self.peek_at(1) == Some('/') => {
                            self.bump();
                            self.bump();
                            depth -= 1;
                        }
                        _ => {
                            self.bump();
                        }
                    }
                }
                self.push(Tk::BlockComment, lo);
                continue;
            }

            // Raw / byte strings: r"..", r#".."#, b"..", br#".."#
            if (c == 'r' || c == 'b') && self.raw_or_byte_string_ahead() {
                self.lex_raw_or_byte_string(lo)?;
                continue;
            }

            // Byte char literal: `b'x'`. Must be checked before identifiers,
            // or the `b` is taken as a one-letter name.
            if c == 'b' && self.peek_at(1) == Some('\'') {
                self.i += 1;
                self.lex_char(lo)?;
                continue;
            }

            // Identifiers and keywords; a raw identifier `r#type` is one
            // token, its text kept as written.
            if c == '_' || c.is_alphabetic() {
                if c == 'r'
                    && self.peek_at(1) == Some('#')
                    && matches!(self.peek_at(2), Some(ch) if ch == '_' || ch.is_alphabetic())
                {
                    self.i += 2;
                }
                while matches!(self.peek(), Some(ch) if ch == '_' || ch.is_alphanumeric()) {
                    self.i += 1;
                }
                self.push(Tk::Ident, lo);
                continue;
            }

            // Numbers
            if c.is_ascii_digit() {
                self.lex_number(lo);
                continue;
            }

            // Char literal vs lifetime
            if c == '\'' {
                if self.is_lifetime() {
                    self.i += 1; // '
                    while matches!(self.peek(), Some(ch) if ch == '_' || ch.is_alphanumeric()) {
                        self.i += 1;
                    }
                    self.push(Tk::Lifetime, lo);
                } else {
                    self.lex_char(lo)?;
                }
                continue;
            }

            if c == '"' {
                self.lex_string(lo)?;
                continue;
            }

            self.lex_punct(lo)?;
        }
        Ok(self.toks)
    }

    fn raw_or_byte_string_ahead(&self) -> bool {
        let mut n = 1;
        if self.peek() == Some('b') && self.peek_at(1) == Some('r') {
            n = 2;
        } else if self.peek() == Some('b') {
            return self.peek_at(1) == Some('"');
        } else if self.peek() != Some('r') {
            return false;
        }
        let mut k = n;
        while self.peek_at(k) == Some('#') {
            k += 1;
        }
        self.peek_at(k) == Some('"')
    }

    fn lex_raw_or_byte_string(&mut self, lo: usize) -> Result<(), LexError> {
        // consume prefix letters
        while matches!(self.peek(), Some('r') | Some('b')) {
            self.i += 1;
        }
        let mut hashes = 0usize;
        while self.peek() == Some('#') {
            hashes += 1;
            self.i += 1;
        }
        if self.peek() != Some('"') {
            // `b"..."` with no hashes falls through to a normal escaped string
            return self.lex_string(lo);
        }
        self.bump(); // opening quote
        if hashes == 0 {
            // r"..." — no escapes
            loop {
                match self.bump() {
                    None => {
                        return Err(LexError {
                            msg: "unterminated raw string".into(),
                            span: Span::new(lo, self.off()),
                        })
                    }
                    Some('"') => break,
                    _ => {}
                }
            }
        } else {
            loop {
                match self.bump() {
                    None => {
                        return Err(LexError {
                            msg: "unterminated raw string".into(),
                            span: Span::new(lo, self.off()),
                        })
                    }
                    Some('"') => {
                        let mut k = 0;
                        while k < hashes && self.peek_at(k) == Some('#') {
                            k += 1;
                        }
                        if k == hashes {
                            self.i += hashes;
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
        self.push(Tk::Str, lo);
        Ok(())
    }

    fn lex_string(&mut self, lo: usize) -> Result<(), LexError> {
        while matches!(self.peek(), Some('b') | Some('r')) {
            self.i += 1;
        }
        self.bump(); // opening "
        loop {
            match self.bump() {
                None => {
                    return Err(LexError {
                        msg: "unterminated string literal".into(),
                        span: Span::new(lo, self.off()),
                    })
                }
                Some('\\') => {
                    self.bump();
                }
                Some('"') => break,
                _ => {}
            }
        }
        self.push(Tk::Str, lo);
        Ok(())
    }

    /// `'a` is a lifetime; `'a'` is a char. Distinguish by looking for the
    /// closing quote.
    fn is_lifetime(&self) -> bool {
        match self.peek_at(1) {
            Some(c) if c == '_' || c.is_alphabetic() => {
                // `'a'` -> char, `'ab` / `'a ` -> lifetime
                self.peek_at(2) != Some('\'')
            }
            _ => false,
        }
    }

    fn lex_char(&mut self, lo: usize) -> Result<(), LexError> {
        self.bump(); // '
        loop {
            match self.bump() {
                None => {
                    return Err(LexError {
                        msg: "unterminated character literal".into(),
                        span: Span::new(lo, self.off()),
                    })
                }
                Some('\\') => {
                    self.bump();
                }
                Some('\'') => break,
                _ => {}
            }
        }
        self.push(Tk::Char, lo);
        Ok(())
    }

    fn lex_number(&mut self, lo: usize) {
        let mut is_float = false;
        if self.peek() == Some('0') && matches!(self.peek_at(1), Some('x') | Some('o') | Some('b')) {
            self.i += 2;
            while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
                self.i += 1;
            }
            self.push(Tk::Int, lo);
            return;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') {
            self.i += 1;
        }
        // A `.` is part of the float only if followed by a digit. `1.foo()` and
        // `1..2` keep the dot as its own token.
        if self.peek() == Some('.') && matches!(self.peek_at(1), Some(c) if c.is_ascii_digit()) {
            is_float = true;
            self.i += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some('e') | Some('E'))
            && matches!(self.peek_at(1), Some(c) if c.is_ascii_digit() || c == '+' || c == '-')
        {
            is_float = true;
            self.i += 2;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') {
                self.i += 1;
            }
        }
        // suffix: i32, f64, usize ...
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            if matches!(self.peek(), Some('f')) {
                is_float = true;
            }
            self.i += 1;
        }
        self.push(if is_float { Tk::Float } else { Tk::Int }, lo);
    }

    fn lex_punct(&mut self, lo: usize) -> Result<(), LexError> {
        let c = self.peek().unwrap();
        let c1 = self.peek_at(1);
        let c2 = self.peek_at(2);

        // `<-` and `<|` before `<`, so they win over comparison. Rust requires
        // a space in `x < -1`, and a comparison against a closure is never
        // valid, so neither is ambiguous.
        if c == '<' && c1 == Some('-') {
            self.i += 2;
            self.push(Tk::LArrow, lo);
            return Ok(());
        }
        if self.mode == Mode::Harsh && c == '<' && c1 == Some('|') {
            self.i += 2;
            self.push(Tk::PipeBack, lo);
            return Ok(());
        }
        if self.mode == Mode::Harsh && c == '|' && c1 == Some('>') {
            self.i += 2;
            self.push(Tk::PipeFwd, lo);
            return Ok(());
        }
        // `..=` / `..`
        if c == '.' && c1 == Some('.') {
            self.i += if c2 == Some('=') { 3 } else { 2 };
            self.push(Tk::DotDot, lo);
            return Ok(());
        }
        // `.0` tuple index — a path segment can never start with a digit.
        if c == '.' && matches!(c1, Some(d) if d.is_ascii_digit()) {
            self.i += 1;
            while matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
                self.i += 1;
            }
            self.push(Tk::TupleIdx, lo);
            return Ok(());
        }
        if c == '.' {
            // Plain substitution, both directions:
            //     Rust `::`  <->  Harsh `.`
            //     Rust `.`   <->  Harsh `<-`
            // No context, no exceptions. The three cases handled above -- float
            // literals, `.0` tuple indexes and `..` ranges -- are different
            // tokens entirely, not special cases of this one.
            self.i += 1;
            self.push(if self.mode == Mode::Rust { Tk::LArrow } else { Tk::Dot }, lo);
            return Ok(());
        }
        if c == ':' && c1 == Some(':') {
            // `::` is Rust's spelling. In Harsh the path separator is `.`, and
            // allowing both would be a second spelling for one thing -- the
            // same drift that a permissive rule caused before.
            if self.mode == Mode::Harsh {
                return Err(LexError {
                    msg: "`::` is not valid in Harsh; the path separator is `.`".into(),
                    span: Span::new(lo, lo + 2),
                });
            }
            self.i += 2;
            self.push(Tk::PathSep, lo);
            return Ok(());
        }
        if c == '=' && c1 == Some('>') {
            self.i += 2;
            self.push(Tk::FatArrow, lo);
            return Ok(());
        }

        // Multi-char operators kept verbatim as Punct.
        const THREE: [&str; 2] = ["<<=", ">>="];
        const TWO: [&str; 15] = [
            "==", "!=", "<=", ">=", "&&", "||", "<<", ">>", "+=", "-=", "*=", "/=", "%=", "^=", "->",
        ];
        let rest: String = [Some(c), c1, c2].iter().flatten().collect();
        for op in THREE {
            if rest.starts_with(op) {
                self.i += 3;
                self.push(Tk::Punct, lo);
                return Ok(());
            }
        }
        for op in TWO {
            if rest.starts_with(op) {
                self.i += 2;
                self.push(Tk::Punct, lo);
                return Ok(());
            }
        }

        self.i += 1;
        let kind = match c {
            ':' => Tk::Colon,
            ';' => Tk::Semi,
            ',' => Tk::Comma,
            '=' => Tk::Eq,
            '<' => Tk::Lt,
            '>' => Tk::Gt,
            '#' => Tk::Hash,
            '\\' => Tk::Backslash,
            '(' | '[' | '{' => Tk::Open(c),
            ')' | ']' | '}' => Tk::Close(c),
            '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '!' | '?' | '@' | '$' | '~' => Tk::Punct,
            _ => {
                return Err(LexError {
                    msg: format!("unexpected character `{}`", c),
                    span: Span::new(lo, self.off()),
                })
            }
        };
        self.push(kind, lo);
        Ok(())
    }
}
