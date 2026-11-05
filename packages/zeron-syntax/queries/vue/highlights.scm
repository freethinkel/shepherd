; Only leaf nodes: a capture on a container (upstream's `(component) @tag`) outranks every injected span inside it.
(tag_name) @tag
(attribute_name) @attribute
(attribute (quoted_attribute_value) @string)
(attribute (attribute_value) @string)
(comment) @comment
(directive_name) @keyword
(directive_argument) @attribute
(directive_modifier) @attribute
(interpolation ["{{" "}}"] @punctuation.special)
["<" ">" "</" "/>"] @punctuation.bracket
"=" @operator
