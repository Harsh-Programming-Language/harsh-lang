/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/**
 * Harsh — Rust with indentation instead of braces.
 *
 * The block structure comes from an external scanner emitting INDENT, DEDENT
 * and NEWLINE, the same approach tree-sitter-python uses. Expressions, types
 * and patterns are deliberately loose: this grammar exists for highlighting
 * and outline, not for type checking, so it accepts more than the transpiler
 * does rather than producing ERROR nodes on valid code.
 */
module.exports = grammar({
  name: 'harsh',

  externals: $ => [$._newline, $._indent, $._dedent],
  // Newlines are claimed by the external scanner wherever layout gives them
  // meaning; inside a bracket group the scanner declines them, and they are
  // then ordinary whitespace -- which is what makes a group span lines.
  extras: $ => [/[ \t\r\n]/, $.line_comment, $.block_comment],
  word: $ => $.identifier,

  rules: {
    source_file: $ => repeat($._statement),

    _statement: $ => choice($.block, $.line, $._newline),

    // A line that ends in `:` or `=>` opens an indented block.
    block: $ => seq(
      field('header', $.header),
      $._newline,
      $._indent,
      repeat($._statement),
      $._dedent,
    ),

    header: $ => prec.dynamic(1, seq(
      repeat1($._token),
      field('opener', choice(':', '=>')),
    )),

    line: $ => seq(repeat1($._token), $._newline),

    _token: $ => choice(
      $.attribute,
      $.keyword,
      $.identifier,
      $.lifetime,
      $.number,
      $.string,
      $.char,
      $.macro_name,
      $.metavariable,
      $.apply_nothing,
      $.operator,
      $.punctuation,
      $.group,
    ),

    group: $ => choice(
      seq('(', repeat(choice($._token, $.paren_block)), ')'),
      seq('[', repeat($._token), ']'),
      seq('{', repeat($._token), '}'),
    ),

    // Parens are transparent to layout: a `:` ending a line inside one opens
    // a block, and the matching `)` closes it wherever it sits -- on the
    // body's last line, or on a line of its own.
    paren_block: $ => prec.dynamic(2, seq(
      field('opener', ':'),
      $._newline,
      $._indent,
      repeat($._statement),
      optional($.last_line),
      $._dedent,
    )),

    // A body line ended by the closing `)` rather than a newline.
    last_line: $ => prec.right(repeat1($._token)),

    attribute: $ => seq('#', optional('!'), '[', repeat($._token), ']'),

    keyword: $ => choice(
      'as', 'async', 'await', 'break', 'const', 'continue', 'crate', 'do',
      'dyn', 'else', 'enum', 'extern', 'fn', 'for', 'if', 'impl', 'in', 'let',
      'loop', 'match', 'mod', 'move', 'mut', 'pub', 'ref', 'return', 'self',
      'Self', 'static', 'struct', 'super', 'trait', 'type', 'union', 'unsafe',
      'use', 'where', 'while',
    ),

    // Harsh's own operators, then Rust's.
    operator: $ => choice(
      '<-',   // field access and method call
      '<|',   // backward application pipe
      '|>',   // forward application pipe
      '\\',   // a struct's field list: `Point\ x = 1`, `struct P\ x: f64`, the pattern `P\ x, ..`
      '->', '..=', '..', '::', '&&', '||', '==', '!=', '<=', '>=',
      '<<', '>>', '+=', '-=', '*=', '/=', '%=', '^=', '&=', '|=',
      '+', '-', '*', '/', '%', '&', '|', '^', '!', '=', '<', '>', '?', '@',
      prec(-1, '=>'),
    ),

    // `$a`, `$crate` inside a `macro_rules!` body: Rust's metavariable sigil.
    metavariable: $ => token(seq('$', /[A-Za-z_][A-Za-z0-9_]*/)),

    // A bare `$` is Harsh's apply-to-nothing suffix: `f$`, `fn main$:`,
    // `s <- len$`. (A macro body's `$(` lexes the same way and is harmless.)
    apply_nothing: $ => '$',

    macro_name: $ => token(seq(/[A-Za-z_][A-Za-z0-9_]*/, '!')),

    punctuation: $ => prec(-1, choice(',', ';', ':', '.')),

    identifier: $ => /[A-Za-z_][A-Za-z0-9_]*/,
    lifetime: $ => /'[A-Za-z_][A-Za-z0-9_]*/,
    number: $ => /[0-9][0-9_]*(\.[0-9][0-9_]*)?([eE][+-]?[0-9_]+)?[A-Za-z0-9_]*/,

    string: $ => token(choice(
      // An escape may be a backslash-newline: the line-continued string.
      seq('"', repeat(choice(/[^"\\]/, /\\(.|\n)/)), '"'),
      seq('r', repeat('#'), '"', /[^"]*/, '"', repeat('#')),
    )),
    char: $ => token(seq("'", choice(/[^'\\]/, /\\./), "'")),

    line_comment: $ => token(seq('//', /[^\n]*/)),
    block_comment: $ => token(seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/')),
  },
});
