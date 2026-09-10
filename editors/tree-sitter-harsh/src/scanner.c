/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/* External scanner for Harsh's layout.
 *
 * Emits NEWLINE, INDENT and DEDENT the way tree-sitter-python's does, with two
 * differences that follow from Harsh's own rules:
 *
 *   - Indentation is suppressed inside (), [] and {}, so a bracketed region
 *     spans lines freely.
 *   - A more-indented line only opens a block when the previous line ended in
 *     `:` or `=>`; otherwise it continues the current logical line. The parser
 *     signals which it expects through the valid-symbols mask, so the scanner
 *     just reports what it sees.
 */
#include <tree_sitter/parser.h>
#include <stdlib.h>
#include <string.h>

enum TokenType { NEWLINE, INDENT, DEDENT };

typedef struct {
  uint16_t len;
  uint16_t cap;
  uint16_t *cols;
  uint16_t depth;   /* bracket nesting */
  int16_t pending;  /* column seen after a newline, -1 when none */
} Scanner;

void *tree_sitter_harsh_external_scanner_create(void) {
  Scanner *s = calloc(1, sizeof(Scanner));
  s->cap = 16;
  s->cols = calloc(s->cap, sizeof(uint16_t));
  s->cols[0] = 0;
  s->len = 1;
  s->pending = -1;
  return s;
}

void tree_sitter_harsh_external_scanner_destroy(void *p) {
  Scanner *s = p;
  free(s->cols);
  free(s);
}

unsigned tree_sitter_harsh_external_scanner_serialize(void *p, char *buf) {
  Scanner *s = p;
  unsigned n = 0;
  buf[n++] = (char)(s->depth & 0xff);
  for (uint16_t i = 0; i < s->len && n + 2 <= TREE_SITTER_SERIALIZATION_BUFFER_SIZE; i++) {
    buf[n++] = (char)(s->cols[i] & 0xff);
    buf[n++] = (char)(s->cols[i] >> 8);
  }
  return n;
}

void tree_sitter_harsh_external_scanner_deserialize(void *p, const char *buf, unsigned n) {
  Scanner *s = p;
  s->len = 1;
  s->cols[0] = 0;
  s->depth = 0;
  if (n == 0) return;
  unsigned i = 0;
  s->depth = (uint8_t)buf[i++];
  while (i + 1 < n && s->len < s->cap) {
    s->cols[s->len++] = (uint8_t)buf[i] | ((uint8_t)buf[i + 1] << 8);
    i += 2;
  }
}

static void push(Scanner *s, uint16_t col) {
  if (s->len == s->cap) {
    s->cap *= 2;
    s->cols = realloc(s->cols, s->cap * sizeof(uint16_t));
  }
  s->cols[s->len++] = col;
}

bool tree_sitter_harsh_external_scanner_scan(void *p, TSLexer *lexer,
                                             const bool *valid) {
  Scanner *s = p;

  /* A column recorded by an earlier call still has indent tokens owed against
   * it. Emitting NEWLINE consumes the line break, so the indentation it
   * introduced has to be remembered rather than re-scanned. */
  if (s->pending >= 0) {
    uint16_t cur = s->cols[s->len - 1];
    uint16_t col = (uint16_t)s->pending;
    if (valid[INDENT] && col > cur) {
      push(s, col);
      s->pending = -1;
      lexer->result_symbol = INDENT;
      return true;
    }
    if (valid[DEDENT] && col < cur && s->len > 1) {
      s->len--;
      /* More levels may still need closing. */
      if (s->cols[s->len - 1] <= col) s->pending = -1;
      lexer->result_symbol = DEDENT;
      return true;
    }
    s->pending = -1;
  }

  if (lexer->eof(lexer)) {
    if (valid[DEDENT] && s->len > 1) {
      s->len--;
      lexer->result_symbol = DEDENT;
      return true;
    }
    return false;
  }

  bool saw_newline = false;
  uint16_t col = 0;

  for (;;) {
    if (lexer->lookahead == '\n') {
      saw_newline = true;
      col = 0;
      lexer->advance(lexer, true);
    } else if (lexer->lookahead == ' ') {
      col++;
      lexer->advance(lexer, true);
    } else if (lexer->lookahead == '\t') {
      col = (col / 4 + 1) * 4;
      lexer->advance(lexer, true);
    } else if (lexer->lookahead == '\r') {
      lexer->advance(lexer, true);
    } else {
      break;
    }
  }

  /* A `)` with no newline before it, where the parser would accept a
   * DEDENT, closes a block that was opened inside the paren: the body's last
   * line ends here. An ordinary group never has DEDENT valid, so no paren
   * tracking is needed -- the parser's valid-symbol mask already says which
   * case this is. */
  if (!saw_newline) {
    if (lexer->lookahead == ')' && valid[DEDENT] && s->len > 1) {
      s->len--;
      lexer->result_symbol = DEDENT;
      return true;
    }
    return false;
  }

  uint16_t cur = s->cols[s->len - 1];
  if (col != cur) s->pending = (int16_t)col;

  if (valid[NEWLINE]) {
    lexer->result_symbol = NEWLINE;
    return true;
  }
  if (valid[INDENT] && col > cur) {
    push(s, col);
    s->pending = -1;
    lexer->result_symbol = INDENT;
    return true;
  }
  if (valid[DEDENT] && col < cur && s->len > 1) {
    s->len--;
    if (s->cols[s->len - 1] <= col) s->pending = -1;
    lexer->result_symbol = DEDENT;
    return true;
  }
  return false;
}
