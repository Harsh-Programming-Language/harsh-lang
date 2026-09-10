; Items appear in Zed's outline: the header of any block that declares one.
(block
  header: (header (keyword) @context (identifier) @name)
  (#match? @context "^(fn|struct|enum|trait|impl|mod|union|type)$")) @item
