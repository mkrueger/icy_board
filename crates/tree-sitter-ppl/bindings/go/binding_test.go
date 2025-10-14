package tree_sitter_ppl_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_ppl "github.com/mkrueger/icy_board//bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_ppl.Language())
	if language == nil {
		t.Errorf("Error loading PCBoard Programming Language grammar")
	}
}
