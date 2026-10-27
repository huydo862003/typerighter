-- Checkbox renderer: icon replacement + toggle command
local state = require("typedown.render.state")

local M = {}
M.name = "checkboxes"

local UNCHECKED_ICON = "󰄱"
local CHECKED_ICON   = "󰄲"

local query

local function get_query()
  if not query then
    query = vim.treesitter.query.parse("typedown_md", [[
      (list_item (checkbox_unchecked) @unchecked)
      (list_item (checkbox_checked)   @checked)
    ]])
  end
  return query
end

function M.render(buf, md_tree)
  local ns = state.get_namespace(M.name)
  state.clear(buf, M.name)
  local cursor_line = state.get_cursor_line(buf)

  local q = get_query()
  for id, node in q:iter_captures(md_tree:root(), buf) do
    local row, col, _, end_col = node:range()
    if row ~= cursor_line then
      local capture = q.captures[id]
      local icon = capture == "checked" and CHECKED_ICON or UNCHECKED_ICON
      local hl   = capture == "checked"
          and "TypedownRenderCheckboxChecked"
          or "TypedownRenderCheckboxUnchecked"

      vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
        end_col = end_col,
        conceal = "",
      })
      vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
        virt_text     = { { icon, hl } },
        virt_text_pos = "inline",
        priority      = 110,
      })
    end
  end
end

-- Toggle checkbox on current line
function M.toggle()
  local line_nr = vim.api.nvim_win_get_cursor(0)[1]
  local line = vim.api.nvim_buf_get_lines(0, line_nr - 1, line_nr, false)[1]
  if not line then return end

  local new_line = line:gsub("%[( )%]", "[x]", 1)
  if new_line == line then
    new_line = line:gsub("%[[xX]%]", "[ ]", 1)
  end

  if new_line ~= line then
    vim.api.nvim_buf_set_lines(0, line_nr - 1, line_nr, false, { new_line })
  end
end

function M.setup_commands()
  vim.api.nvim_create_user_command("TypedownToggleCheckbox", M.toggle, {
    desc = "Toggle checkbox on current line",
  })
end

return M
