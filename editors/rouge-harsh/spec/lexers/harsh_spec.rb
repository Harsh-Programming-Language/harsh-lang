# -*- coding: utf-8 -*- #
# frozen_string_literal: true

describe Rouge::Lexers::Harsh do
  let(:subject) { Rouge::Lexers::Harsh.new }

  describe 'guessing' do
    include Support::Guessing

    it 'guesses by filename' do
      assert_guess :filename => 'foo.hrs'
    end

    it 'guesses by mimetype' do
      assert_guess :mimetype => 'text/x-harsh'
    end
  end

  describe 'lexing' do
    include Support::Lexing

    it 'lexes member access and the pipes' do
      assert_tokens_equal 'v <- len',
        ['Name', 'v'], ['Text', ' '], ['Operator', '<-'], ['Text', ' '], ['Name', 'len']
      assert_tokens_equal 'x |> f',
        ['Name', 'x'], ['Text', ' '], ['Operator', '|>'], ['Text', ' '], ['Name', 'f']
    end

    it 'lexes a call with no arguments' do
      assert_tokens_equal 'f$', ['Name', 'f'], ['Operator', '$']
    end

    it 'lexes do: as a keyword' do
      assert_tokens_equal 'do:', ['Keyword', 'do'], ['Punctuation', ':']
    end

    it 'lexes macros of both kinds' do
      assert_tokens_equal 'v~ 1',
        ['Name.Decorator', 'v~'], ['Text', ' '], ['Literal.Number.Integer', '1']
      assert_tokens_equal 'println! x',
        ['Name.Decorator', 'println!'], ['Text', ' '], ['Name', 'x']
    end

    it 'lexes the turbofish' do
      assert_tokens_equal 'parse.<i32>',
        ['Name', 'parse'], ['Punctuation', '.'], ['Operator', '<'],
        ['Name.Builtin', 'i32'], ['Operator', '>']
    end

    it 'lexes a metavariable' do
      assert_tokens_equal '$x:expr', ['Name.Variable', '$x:expr']
    end

    it 'lexes a hole inside a DSL body as Harsh' do
      assert_tokens_equal 'p! { a @: x <- y :@ }',
        ['Name.Decorator', 'p!'], ['Text', ' '], ['Punctuation', '{'],
        ['Text', ' a '], ['Punctuation', '@:'], ['Text', ' '], ['Name', 'x'],
        ['Text', ' '], ['Operator', '<-'], ['Text', ' '], ['Name', 'y'],
        ['Text', ' '], ['Punctuation', ':@'], ['Text', ' '], ['Punctuation', '}']
    end

    it 'leaves an escaped hole as text' do
      assert_tokens_equal 'p! { @@: }',
        ['Name.Decorator', 'p!'], ['Text', ' '], ['Punctuation', '{'],
        ['Text', ' '], ['Literal.String.Escape', '@@:'], ['Text', ' '], ['Punctuation', '}']
    end
  end
end
