-- Checkbox renderer: icon replacement + toggle command
local state = require("typedown.render.state")

local M = {}

local UNCHECKED_ICON = "󰄱"
local CHECKED_ICON = "󰄲"

function M.render(buf, md_tree)
  local ns = state.get_namespace("checkboxes")
  state.clear(buf, "checkboxes")
  local cursor_line = state.get_cursor_line(buf)

  M.walk(buf, ns, md_tree:root(), cursor_line)
end

function M.walk(buf, ns, node, cursor_line)
  for child in node:iter_children() do
    if child:type() == "task_list_item" or child:type() == "list_item" then
      -- Look for checkbox pattern in the line text
      local row = child:range()
      if row ~= cursor_line then
        local line = vim.api.nvim_buf_get_lines(buf, row, row + 1, false)[1] or ""
        local open = line:find("%[[ xX]%]")
        if open then
          local char = line:sub(open + 1, open + 1)
          local is_checked = char == "x" or char == "X"
          local icon = is_checked and CHECKED_ICON or UNCHECKED_ICON
          local hl = is_checked and "TypedownRenderCheckboxChecked" or "TypedownRenderCheckboxUnchecked"

          vim.api.nvim_buf_set_extmark(buf, ns, row, open - 1, {
            end_col = open + 2,
            conceal = "",
          })
          vim.api.nvim_buf_set_extmark(buf, ns, row, open - 1, {
            virt_text = { { icon, hl } },
            virt_text_pos = "inline",
            priority = 110,
          })
        end
      end
    end
    M.walk(buf, ns, child, cursor_line)
  end
end

-- Toggle checkbox on current line: [ ] <-> [x]
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
