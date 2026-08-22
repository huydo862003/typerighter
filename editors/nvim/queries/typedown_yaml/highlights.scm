; Keys
((property_key (identifier) @property)
  (#set! priority 101))
(reserved_key) @keyword

; Type values
(type_value "!type" @keyword)

; Values
(number) @number
(boolean) @boolean
(string) @string
(escape_sequence) @string.escape

; Types
(primitive_type) @type.builtin
(list_type "list" @type.builtin)
(dict_type "dict" @type.builtin)
(fixed_key_dict_type) @type
(union_type) @type
((fixed_key_entry key: (identifier) @keyword)
  (#set! priority 101))

; list[T] and dict[K, V] as index expressions
((index_expression
  (expression (identifier) @type.builtin))
  (#any-of? @type.builtin "list" "dict")
  (#set! priority 101))
((index_expression
  (expression (identifier) @type.builtin) .
  (expression (identifier) @type))
  (#any-of? @type.builtin "list" "dict")
  (#set! priority 101))

; Expressions
(identifier) @variable
(self_expression) @variable.builtin
(fref "fref" @function.builtin)
(fref) @function.call
(tag_operator) @operator
(access_expression "." @punctuation.delimiter)
(dict_entry key: (identifier) @property)

; Binary operators
(binary_expression "+" @operator)
(binary_expression "-" @operator)
(binary_expression "*" @operator)
(binary_expression "/" @operator)
(binary_expression "%" @operator)
(binary_expression "**" @operator)
(binary_expression "==" @operator)
(binary_expression "!=" @operator)
(binary_expression "<" @operator)
(binary_expression ">" @operator)
(binary_expression "<=" @operator)
(binary_expression ">=" @operator)
(binary_expression "||" @keyword.operator)
(binary_expression "&&" @keyword.operator)

; Unary operators
(unary_expression "~" @operator)

; Postfix operators
(postfix_expression "?" @operator)
(optional_type "?" @operator)

; Function calls
(call_expression (expression (identifier) @function.call))
(call_expression (expression (access_expression (identifier) @function.call)))

; Interpolation
(interpolation "$" @punctuation.special)
(interpolation "{" @punctuation.bracket)
(interpolation "}" @punctuation.bracket)

; Comments
(comment) @comment

; Punctuation
":" @punctuation.delimiter
"-" @punctuation.delimiter
"," @punctuation.delimiter
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"(" @punctuation.bracket
")" @punctuation.bracket
; Block scalars
(block_scalar_content) @string

; Spell checking
(source_file) @nospell
