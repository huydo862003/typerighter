((inline) @injection.content
  (#set! injection.language "typedown_md_inline"))

; Table cell content gets inline parsing (bold, code, links, interpolations)
((pipe_table_cell) @injection.content
  (#set! injection.language "typedown_md_inline"))

(fenced_code_block
  (language) @injection.language
  (code_fence_content) @injection.content)

((math_block_content) @injection.content
  (#set! injection.language "latex"))
