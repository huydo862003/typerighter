-- Smart action: single keybind that does the contextually correct thing
-- On a link/fref -> follow it (go to definition)
-- On a checkbox -> toggle state
-- On a toggle list -> fold/unfold
-- Otherwise -> normal <CR>

local M = {}

-- Check if cursor is on an fref or link and follow it
local function try_follow_link()
	local line = vim.api.nvim_get_current_line()
	local col = vim.api.nvim_win_get_cursor(0)[2] + 1 -- 1-indexed

	-- Check if cursor is inside fref("...") or ${fref("...")}
	-- Find all fref occurrences on the line and check if cursor is inside one
	local search_start = 1
	while true do
		local m_start, m_end = line:find('fref%("([^"]+)"%)', search_start)
		if not m_start then
			break
		end
		if col >= m_start and col <= m_end then
			-- Cursor is inside this fref - trigger go-to-definition
			vim.lsp.buf.definition()
			return true
		end
		search_start = m_end + 1
	end

	-- Check if cursor is on a markdown link [text](url) or media ![alt](url)
	search_start = 1
	while true do
		local m_start, m_end, url = line:find("!?%[.-%]%((.-)%)", search_start)
		if not m_start then
			break
		end
		if col >= m_start and col <= m_end then
			if url:match("^https?://") then
				vim.ui.open(url)
			else
				-- Internal link - use LSP for proper file resolution
				vim.lsp.buf.definition()
			end
			return true
		end
		search_start = m_end + 1
	end

	return false
end

-- Check if cursor is on a checkbox line and toggle it
local function try_toggle_checkbox()
	local line = vim.api.nvim_get_current_line()
	if line:match("^%s*[-*+] %[.%]") then
		require("typedown.render.checkboxes").toggle()
		return true
	end
	return false
end

-- Check if cursor is on a toggle list item and fold/unfold it
local function try_toggle_fold()
	local line = vim.api.nvim_get_current_line()
	if line:match("^%s*> ") then
		require("typedown.render.toggle-list").toggle()
		return true
	end
	return false
end

function M.action()
	if try_follow_link() then
		return
	end
	if try_toggle_checkbox() then
		return
	end
	if try_toggle_fold() then
		return
	end
	-- Fallback: normal <CR>
	vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<CR>", true, false, true), "n", false)
end

function M.setup(buf)
	vim.keymap.set("n", "<CR>", M.action, {
		buffer = buf,
		desc = "Typedown smart action: follow link, toggle checkbox, or fold",
	})
end

return M
