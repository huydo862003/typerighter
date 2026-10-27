-- Table renderer: box-drawing borders
local state = require("typedown.render.state")

local M = {}
M.name = "tables"

-- Box-drawing characters
local CHARS = {
  horizontal = "─",
  vertical = "│",
  top_left = "┌",
  top_right = "┐",
  bottom_left = "└",
  bottom_right = "┘",
  cross = "┼",
  top_tee = "┬",
  bottom_tee = "┴",
  left_tee = "├",
  right_tee = "┤",
}

function M.render(buf, md_tree)
  local ns = state.get_namespace("tables")
  state.clear(buf, "tables")
  local cursor_line = state.get_cursor_line(buf)

  for node in md_tree:root():iter_children() do
    M.render_node(buf, ns, node, cursor_line)
  end
end

function M.render_node(buf, ns, node, cursor_line)
  if node:type() == "section" then
    for child in node:iter_children() do
      M.render_node(buf, ns, child, cursor_line)
    end
    return
  end

  if node:type() ~= "pipe_table" then return end

  local start_row, _, end_row = node:range()

  -- Replace pipe characters with box-drawing vertical lines
  for row = start_row, end_row - 1 do
    if row == cursor_line then goto continue end

    local line = vim.api.nvim_buf_get_lines(buf, row, row + 1, false)[1] or ""

    -- Find the delimiter row (contains only |, -, :, spaces)
    local is_delimiter = line:match("^[|%-%s:]+$") ~= nil

    for col = 0, #line - 1 do
      if line:sub(col + 1, col + 1) == "|" then
        vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
          end_col = col + 1,
          conceal = "",
        })
        vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
          virt_text = { { CHARS.vertical, "TypedownRenderTableBorder" } },
          virt_text_pos = "inline",
          priority = 110,
        })
      end
    end

    -- Replace delimiter row dashes with horizontal lines
    if is_delimiter then
      for col = 0, #line - 1 do
        local char = line:sub(col + 1, col + 1)
        if char == "-" or char == ":" then
          vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
            end_col = col + 1,
            conceal = "",
          })
          vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
            virt_text = { { CHARS.horizontal, "TypedownRenderTableBorder" } },
            virt_text_pos = "inline",
            priority = 110,
          })
        end
      end
    end

    -- Header row gets bold highlight
    if row == start_row then
      vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
        end_row = row + 1,
        hl_group = "TypedownRenderTableHeader",
        hl_eol = true,
        priority = 80,
      })
    end

    ::continue::
  end
end

return M
