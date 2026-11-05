; tree-sitter-graphql ships no queries; written against its node types.
(comment) @comment
(description (string_value) @comment)
(string_value) @string
(int_value) @number
(float_value) @number
(boolean_value) @boolean
(null_value) @constant.builtin
(enum_value) @constant
(variable) @variable.parameter
(directive "@" @attribute (name) @attribute)
(directive_definition "@" @attribute (name) @attribute)
(directive_location) @constant.builtin
(named_type (name) @type)
(object_type_definition (name) @type)
(interface_type_definition (name) @type)
(input_object_type_definition (name) @type)
(enum_type_definition (name) @type)
(union_type_definition (name) @type)
(scalar_type_definition (name) @type)
(fragment_name) @function
(operation_definition (name) @function)
(field_definition (name) @property)
(input_value_definition (name) @variable.parameter)
(argument (name) @variable.parameter)
(object_field (name) @property)
(alias (name) @property)
(field (name) @property)
(operation_type) @keyword
[
  "query" "mutation" "subscription" "fragment" "on" "type" "interface" "union"
  "enum" "input" "scalar" "schema" "extend" "implements" "directive" "repeatable"
] @keyword
["!" "=" "|" "&" "..."] @operator
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
[":" "$" "@"] @punctuation.delimiter
