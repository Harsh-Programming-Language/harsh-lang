package tree_sitter_harsh_test

import (
	"testing"

	tree_sitter "github.com/smacker/go-tree-sitter"
	"github.com/tree-sitter/tree-sitter-harsh"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_harsh.Language())
	if language == nil {
		t.Errorf("Error loading Harsh grammar")
	}
}
