; Emphasis
(emphasis) @emphasis
(strong_emphasis) @emphasis.strong

; Code and math spans
(code_span_delimiter) @punctuation.delimiter
(code_span_content) @text.literal
(math_span_delimiter) @punctuation.delimiter
(math_span_content) @text.literal

; Links
(inline_link "[" @punctuation.delimiter)
(inline_link "]" @punctuation.delimiter)
(inline_link "(" @punctuation.delimiter)
(inline_link ")" @punctuation.delimiter)
(link_text) @link_text
(link_destination) @link_uri

; Images
(image "!" @punctuation.special)
(image "[" @punctuation.delimiter)
(image "]" @punctuation.delimiter)
(image "(" @punctuation.delimiter)
(image ")" @punctuation.delimiter)
(image_alt) @link_text

; Footnotes
(footnote_reference "[" @punctuation.delimiter)
(footnote_reference "^" @punctuation.special)
(footnote_reference "]" @punctuation.delimiter)
(footnote_label) @link_text

; Citations
(citation "[" @punctuation.delimiter)
(citation "@" @punctuation.special)
(citation "]" @punctuation.delimiter)
(citation_key) @link_text

; Expressions
(identifier) @variable
(self_expression) @variable.special
(number) @number
(boolean) @boolean
(string) @string
(escape_sequence) @string.escape
(fref "fref" @function)
(fref) @function
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
(binary_expression "||" @keyword)
(binary_expression "&&" @keyword)

; Unary operators
(unary_expression "~" @operator)

; Postfix operators
(postfix_expression "?" @operator)

; Function calls
(call_expression (expression (identifier) @function))
(call_expression (expression (access_expression (identifier) @function)))

; Interpolation
(interpolation "$" @punctuation.special)
(interpolation "{" @punctuation.bracket)
(interpolation "}" @punctuation.bracket)

; Punctuation
(parenthesized_expression "(" @punctuation.bracket)
(parenthesized_expression ")" @punctuation.bracket)
(list_expression "[" @punctuation.bracket)
(list_expression "]" @punctuation.bracket)
(dict_expression "{" @punctuation.bracket)
(dict_expression "}" @punctuation.bracket)
(index_expression "[" @punctuation.bracket)
(index_expression "]" @punctuation.bracket)
"," @punctuation.delimiter

; Misc
(hard_line_break) @string.escape
(backslash_escape) @string.escape
