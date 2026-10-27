-- Anti-conceal: show raw markdown on cursor line, rendered everywhere else
-- Triggers re-render when cursor moves to a different line

local state = require("typedown.render.state")

local M = {}

function M.on_cursor_moved(buf)
  local cursor = vim.api.nvim_win_get_cursor(0)
  local new_line = cursor[1] - 1 -- 0-indexed
  local old_line = state.get_cursor_line(buf)

  if new_line ~= old_line then
    state.set_cursor_line(buf, new_line)
    return true -- needs re-render
  end
  return false
end

function M.on_insert_enter(buf)
  -- In insert mode, show raw text everywhere (disable rendering)
  state.set_cursor_line(buf, -2) -- special value: all lines are "cursor line"
  return true
end

function M.on_insert_leave(buf)
  local cursor = vim.api.nvim_win_get_cursor(0)
  state.set_cursor_line(buf, cursor[1] - 1)
  return true
end

return M
