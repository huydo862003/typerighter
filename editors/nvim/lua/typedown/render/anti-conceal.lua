-- Anti-conceal: show raw markdown near cursor, rendered everywhere else
-- Supports configurable range (above/below) and visual mode ranges

local state = require("typedown.render.state")

local M = {}

function M.on_cursor_moved(buf)
	local cursor = vim.api.nvim_win_get_cursor(0)
	local new_line = cursor[1] - 1 -- 0-indexed
	local old_line = state.get_cursor_line(buf)

	-- Also handle visual mode: reveal the full selection range
	local mode = vim.fn.mode()
	state.set_mode(buf, mode)

	if new_line ~= old_line then
		state.set_cursor_line(buf, new_line)
		return true -- needs re-render
	end
	return false
end

function M.on_insert_enter(buf)
	state.set_mode(buf, "i")
	state.set_cursor_line(buf, -2) -- special: all lines are "cursor line"
	return true
end

function M.on_insert_leave(buf)
	state.set_mode(buf, "n")
	local cursor = vim.api.nvim_win_get_cursor(0)
	state.set_cursor_line(buf, cursor[1] - 1)
	return true
end

function M.on_mode_changed(buf)
	local mode = vim.fn.mode()
	local old_mode = state.get_mode(buf)
	state.set_mode(buf, mode)

	if mode ~= old_mode then
		-- Update cursor position on mode change
		local ok, cursor = pcall(vim.api.nvim_win_get_cursor, 0)
		if ok then
			state.set_cursor_line(buf, cursor[1] - 1)
		end
		return true
	end
	return false
end

return M
