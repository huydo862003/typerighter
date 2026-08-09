(section) @class.around

(fenced_code_block) @function.around
(fenced_code_block
  (code_fence_content) @function.inside)

(math_block) @function.around
(math_block
  (math_block_content) @function.inside)

(block_quote) @function.around
(list_item) @function.around
(toggle_list_item) @function.around
(container_block) @function.around
(pipe_table) @function.around
