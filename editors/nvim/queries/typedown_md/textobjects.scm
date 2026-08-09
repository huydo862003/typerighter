; Sections (class-level navigation)
(section) @class.outer

; Code blocks
(fenced_code_block) @block.outer
(fenced_code_block
  (code_fence_content) @block.inner)

; Math blocks
(math_block) @block.outer
(math_block
  (math_block_content) @block.inner)

; Block quotes
(block_quote) @block.outer

; List items
(list_item) @block.outer

; Toggle list items
(toggle_list_item) @block.outer

; Containers
(container_block) @block.outer

; Tables
(pipe_table) @block.outer

; Comments (adjacent line comments)
(comment)+ @comment.outer
