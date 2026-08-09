; Headings
(atx_heading
  (atx_h1_marker) @title
  (#set! "priority" 110)) @title
(atx_heading
  (atx_h2_marker) @title
  (#set! "priority" 110)) @title
(atx_heading
  (atx_h3_marker) @title
  (#set! "priority" 110)) @title
(atx_heading
  (atx_h4_marker) @title
  (#set! "priority" 110)) @title
(atx_heading
  (atx_h5_marker) @title
  (#set! "priority" 110)) @title
(atx_heading
  (atx_h6_marker) @title
  (#set! "priority" 110)) @title

; Code blocks
(fenced_code_block_delimiter) @punctuation.special
(language) @label
(code_fence_content) @text.literal

; Math blocks
(math_block_delimiter) @punctuation.special
(math_block_content) @text.literal

; Block quotes
(block_quote_marker) @punctuation.special

; Toggle lists
(toggle_list_marker) @punctuation.special

; Lists
(list_marker_minus) @punctuation.list_marker
(list_marker_star) @punctuation.list_marker
(list_marker_dot) @punctuation.list_marker

; Containers
(container_block_delimiter) @punctuation.special
(container_type) @label

; Tables
(pipe_table_header) @title
(pipe_table_delimiter_row) @punctuation.delimiter
(pipe_table_cell) @text
