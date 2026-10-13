# -*- coding: utf-8 -*- #
# frozen_string_literal: true

require_relative 'rust'

module Rouge
  module Lexers
    # Harsh is Rust written with indentation instead of braces
    # (https://harsh-lang.com). It keeps Rust's tokens and adds a few of its
    # own, so this lexer is Rust's with those prepended:
    #
    #   value <- method          member access, Rust's `.`
    #   f$                       a call with no arguments, Rust's `f()`
    #   x |> f    f <| x         the pipes
    #   do:                      a bare block
    #   Point\ x = 1, y = 2      `\` opens a literal's fields
    #   name~ stream             a Harsh macro call
    #   name! args               a Rust macro, applied like a function
    #   view! { <p>{@: x :@}</p> }
    #                            a macro's brace body is its own language;
    #                            the holes `@: … :@` inside it are Harsh
    class Harsh < Rust
      title "Harsh"
      desc "Harsh, Rust with indentation instead of braces (harsh-lang.com)"
      tag 'harsh'
      aliases 'hrs'
      filenames '*.hrs'
      mimetypes 'text/x-harsh'

      # Rust's lexer claims a `rustc` shebang; Harsh has none of its own.
      def self.detect?(text)
        false
      end

      id = /[\p{XID_Start}_]\p{XID_Continue}*/

      prepend :root do
        # A `macro_rules!` body is Rust's own zone, braces and all.
        rule %r/\bmacro_rules!/, Name::Decorator, :macro_rules

        # A Rust macro's brace body: the DSL's text, with Harsh holes.
        rule %r/(#{id}!)(\s*)(\{)/ do
          groups Name::Decorator, Text, Punctuation
          push :dsl
        end

        # A Rust macro applied like a function, `println! "{}" x`: no
        # delimiters to track, unlike Rust's `name!(…)`.
        rule %r/#{id}!(?!=)/, Name::Decorator

        # A Harsh macro, `v~ 1 2 3`, `macro_rules~ name`.
        rule %r/#{id}~/, Name::Decorator

        # A Harsh macro's metavariables, `($x:expr)`, `$( … )*`.
        rule %r/[$]#{id}(:#{id})?/, Name::Variable
        rule %r/[$](?=\()/, Name::Variable

        # `f$`: a name applied to nothing.
        rule %r/[$]/, Operator

        rule %r/<-/, Operator
        rule %r/<\||\|>/, Operator
        rule %r/\bdo(?=\s*:)/, Keyword
        rule %r/\\/, Punctuation

        # The turbofish is `.<T>`: Rust's `::` is Harsh's `.`.
        rule %r/[.](?=<)/, Punctuation
      end

      prepend :attribute do
        # `#[derive~ Name]`, `#[route~ GET "/"]`
        rule %r/~/, Name::Decorator
      end

      state :dsl do
        rule %r/@@:/, Str::Escape
        rule %r/@:/, Punctuation, :hole
        rule %r/"/, Str, :dsl_string
        rule %r/\{/, Punctuation, :dsl
        rule %r/\}/, Punctuation, :pop!
        rule %r/[^{}"@]+|@/m, Text
      end

      state :dsl_string do
        rule %r/"/, Str, :pop!
        rule %r/@@:/, Str::Escape
        rule %r/@:/, Punctuation, :hole
        rule %r/\\./, Str::Escape
        rule %r/[^"\\@]+|@/m, Str
      end

      state :hole do
        rule %r/:@/, Punctuation, :pop!
        mixin :root
      end
    end
  end
end
