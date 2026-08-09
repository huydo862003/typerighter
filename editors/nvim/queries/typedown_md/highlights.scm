; Headings
(atx_heading
  (atx_h1_marker) @markup.heading.1.marker
  (#set! "priority" 110)) @markup.heading.1
(atx_heading
  (atx_h2_marker) @markup.heading.2.marker
  (#set! "priority" 110)) @markup.heading.2
(atx_heading
  (atx_h3_marker) @markup.heading.3.marker
  (#set! "priority" 110)) @markup.heading.3
(atx_heading
  (atx_h4_marker) @markup.heading.4.marker
  (#set! "priority" 110)) @markup.heading.4
(atx_heading
  (atx_h5_marker) @markup.heading.5.marker
  (#set! "priority" 110)) @markup.heading.5
(atx_heading
  (atx_h6_marker) @markup.heading.6.marker
  (#set! "priority" 110)) @markup.heading.6

; Code blocks
(fenced_code_block) @markup.raw.block
(fenced_code_block_delimiter) @punctuation.delimiter
(language) @label
(line_range_indicator) @comment
(code_fence_content) @markup.raw

; Math blocks
(math_block) @markup.math
(math_block_delimiter) @punctuation.delimiter
(math_block_content) @markup.math

; Block quotes
(block_quote) @markup.quote
(block_quote_marker) @punctuation.special

; Toggle lists
(toggle_list_marker) @punctuation.special

; Lists
(list_marker_minus) @markup.list
(list_marker_star) @markup.list
(list_marker_dot) @markup.list

; Containers
(container_block_delimiter) @punctuation.special
(container_type) @label
(container_prop_block ["{" "}"] @punctuation.bracket)
(container_prop_key) @attribute
(container_prop_item "=" @operator)
(container_prop_number) @number
(container_prop_string) @string
(container_slot_delimiter) @punctuation.special
(container_slot_name) @label

; Tables
(pipe_table_header) @markup.heading
(pipe_table_delimiter_row) @punctuation.delimiter
(pipe_table_cell) @markup.raw
(pipe_table_row "|" @punctuation.special)
(pipe_table_header "|" @punctuation.special)

; Spell checking
(inline) @spell
(code_fence_content) @nospell
(math_block_content) @nospell
(language) @nospell
(line_range_indicator) @nospell
(container_type) @nospell
(container_prop_key) @nospell
(container_slot_name) @nospell

