-- Per-buffer rendering state management
-- Each renderer gets its own extmark namespace for independent clear/update

local M = {}

-- Namespace registry: renderer_name -> namespace_id
local namespaces = {}

-- Per-buffer state: buf -> { cursor_line, attached, mode, tick }
local buf_state = {}

-- Anti-conceal config
M.anti_conceal = {
	above = 0, -- lines above cursor to reveal
	below = 0, -- lines below cursor to reveal
}

-- Modes where rendering is disabled entirely
M.disabled_modes = { "i" }

-- Performance guards
M.max_file_length = 5000 -- skip rendering for files over this many lines
M.debounce_ms = 50 -- debounce interval for text change re-renders

function M.get_namespace(name)
	if not namespaces[name] then
		namespaces[name] = vim.api.nvim_create_namespace("typedown_render_" .. name)
	end
	return namespaces[name]
end

function M.clear(buf, name)
	local ns = M.get_namespace(name)
	vim.api.nvim_buf_clear_namespace(buf, ns, 0, -1)
end

function M.get_buf_state(buf)
	if not buf_state[buf] then
		buf_state[buf] = { cursor_line = -1, attached = false, mode = "n", tick = -1 }
	end
	return buf_state[buf]
end

function M.set_cursor_line(buf, line)
	M.get_buf_state(buf).cursor_line = line
end

function M.get_cursor_line(buf)
	return M.get_buf_state(buf).cursor_line
end

function M.set_mode(buf, mode)
	M.get_buf_state(buf).mode = mode
end

function M.get_mode(buf)
	return M.get_buf_state(buf).mode
end

-- Check if a row should show raw text (anti-conceal range).
-- Elements that set `always_visible = true` bypass this check.
function M.is_hidden(buf, row)
	local s = M.get_buf_state(buf)
	if s.cursor_line < 0 then
		return false
	end -- no cursor info
	local above = M.anti_conceal.above
	local below = M.anti_conceal.below
	return row >= (s.cursor_line - above) and row <= (s.cursor_line + below)
end

-- Check if buffer content has changed since last render.
-- Returns true if changed (and updates the stored tick).
function M.has_changed(buf)
	local s = M.get_buf_state(buf)
	local tick = vim.api.nvim_buf_get_changedtick(buf)
	if tick == s.tick then
		return false
	end
	s.tick = tick
	return true
end

-- Force tick update without checking (used after cursor-only re-renders)
function M.update_tick(buf)
	M.get_buf_state(buf).tick = vim.api.nvim_buf_get_changedtick(buf)
end

-- Check if rendering is disabled for the current mode
function M.is_mode_disabled(buf)
	local mode = M.get_mode(buf)
	for _, m in ipairs(M.disabled_modes) do
		if mode == m then
			return true
		end
	end
	return false
end

function M.mark_attached(buf)
	M.get_buf_state(buf).attached = true
end

function M.is_attached(buf)
	return M.get_buf_state(buf).attached
end

function M.detach(buf)
	buf_state[buf] = nil
	for name in pairs(namespaces) do
		pcall(vim.api.nvim_buf_clear_namespace, buf, namespaces[name], 0, -1)
	end
end

-- Get typedown_md tree objects for a buffer
function M.get_md_trees(buf)
	local parser = vim.treesitter.get_parser(buf, "typedown")
	if not parser then
		return {}
	end
	parser:parse()

	for _, child_tree in pairs(parser:children()) do
		if child_tree:lang() == "typedown_md" then
			return child_tree:trees()
		end
	end
	return {}
end

return M
