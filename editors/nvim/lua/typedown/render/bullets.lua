-- Bullet point renderer: replace -/*/ + with styled icons
local state = require("typedown.render.state")

local M = {}

local BULLET_ICONS = { "●", "○", "◆", "◇", "■", "□" }

function M.render(buf, md_tree)
  local ns = state.get_namespace("bullets")
  state.clear(buf, "bullets")
  local cursor_line = state.get_cursor_line(buf)

  M.walk(buf, ns, md_tree:root(), cursor_line, 0)
end

function M.walk(buf, ns, node, cursor_line, depth)
  for child in node:iter_children() do
    local kind = child:type()
    if kind == "list_marker_minus" or kind == "list_marker_star" or kind == "list_marker_plus" then
      local row, col, _, end_col = child:range()
      if row ~= cursor_line then
        local icon = BULLET_ICONS[((depth) % #BULLET_ICONS) + 1]
        vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
          end_col = end_col,
          conceal = "",
        })
        vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
          virt_text = { { icon, "TypedownRenderBullet" } },
          virt_text_pos = "inline",
          priority = 110,
        })
      end
    elseif kind == "bullet_list" or kind == "ordered_list" then
      M.walk(buf, ns, child, cursor_line, depth + 1)
    else
      M.walk(buf, ns, child, cursor_line, depth)
    end
  end
end

return M
