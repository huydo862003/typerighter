-- Table renderer: conceal pipes and delimiter chars, replace with box-drawing
local state = require("typedown.render.state")

local M = {}
M.name = "tables"

local BORDER = {
  horizontal = "─",
  vertical   = "│",
}

local query

local function get_query()
  if not query then
    query = vim.treesitter.query.parse("typedown_md", [[
      (pipe_table) @table
    ]])
  end
  return query
end

function M.render(buf, md_tree)
  local ns = state.get_namespace(M.name)
  state.clear(buf, M.name)

  local q = get_query()
  for _, node in q:iter_captures(md_tree:root(), buf) do
    M.render_table(buf, ns, node)
  end
end

function M.render_table(buf, ns, node)
  local start_row, _, end_row = node:range()

  -- Ensure conceallevel is set so conceal extmarks work
  for _, win in ipairs(vim.fn.win_findbuf(buf)) do
    if vim.api.nvim_get_option_value("conceallevel", { win = win }) < 2 then
      vim.api.nvim_set_option_value("conceallevel", 2, { win = win })
    end
  end

  for row = start_row, end_row - 1 do
    if not state.is_hidden(buf, row) then
      local line = vim.api.nvim_buf_get_lines(buf, row, row + 1, false)[1] or ""
      local is_delimiter = line:match("^%s*[|%-%s:]+$") ~= nil

      for col_idx = 1, #line do
        local char = line:sub(col_idx, col_idx)
        if char == "|" then
          -- Conceal | and insert │
          vim.api.nvim_buf_set_extmark(buf, ns, row, col_idx - 1, {
            end_col = col_idx,
            conceal = "",
          })
          vim.api.nvim_buf_set_extmark(buf, ns, row, col_idx - 1, {
            virt_text     = { { BORDER.vertical, "TypedownRenderTableBorder" } },
            virt_text_pos = "inline",
            priority      = 110,
          })
        elseif is_delimiter and (char == "-" or char == ":") then
          vim.api.nvim_buf_set_extmark(buf, ns, row, col_idx - 1, {
            end_col = col_idx,
            conceal = "",
          })
          vim.api.nvim_buf_set_extmark(buf, ns, row, col_idx - 1, {
            virt_text     = { { BORDER.horizontal, "TypedownRenderTableBorder" } },
            virt_text_pos = "inline",
            priority      = 110,
          })
        end
      end

      -- Header row bold
      if row == start_row then
        vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
          end_row  = row + 1,
          hl_group = "TypedownRenderTableHeader",
          hl_eol   = true,
          priority = 80,
        })
      end
    end
  end
end

return M
