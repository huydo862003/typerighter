; Emphasis
(emphasis) @markup.italic
(strong_emphasis) @markup.strong

; Code and math spans
(code_span) @markup.raw
(code_span_delimiter) @punctuation.delimiter
(code_span_content) @markup.raw
(math_span) @markup.math
(math_span_delimiter) @punctuation.delimiter
(math_span_content) @markup.math

; Links
(inline_link) @markup.link
(inline_link "[" @punctuation.delimiter)
(inline_link "]" @punctuation.delimiter)
(inline_link "(" @punctuation.delimiter)
(inline_link ")" @punctuation.delimiter)
(link_text) @markup.link.label
(link_destination) @markup.link.url

; Images
(image) @markup.link
(image "!" @punctuation.special)
(image "[" @punctuation.delimiter)
(image "]" @punctuation.delimiter)
(image "(" @punctuation.delimiter)
(image ")" @punctuation.delimiter)
(image_alt) @markup.link.label

; Footnotes
(footnote_reference) @markup.link
(footnote_reference "[" @punctuation.delimiter)
(footnote_reference "^" @punctuation.special)
(footnote_reference "]" @punctuation.delimiter)
(footnote_label) @markup.link.label

; Citations
(citation) @markup.link
(citation "[" @punctuation.delimiter)
(citation "@" @punctuation.special)
(citation "]" @punctuation.delimiter)
(citation_key) @markup.link.label

; Expressions
(identifier) @variable
(self_expression) @variable.builtin
(number) @number
(boolean) @boolean
(string) @string
(escape_sequence) @string.escape
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

; Function calls
(call_expression (expression (identifier) @function.call))
(call_expression (expression (access_expression (identifier) @function.call)))

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

; Spell checking
(code_span) @nospell
(link_destination) @nospell
(citation_key) @nospell
(footnote_label) @nospell
(interpolation) @nospell
(fref) @nospell

; Misc
(hard_line_break) @string.escape
(backslash_escape) @string.escape
