-- Link renderer: underline fref(), markdown links, and media embeds
local state = require("typedown.render.state")

local M = {}
M.name = "links"

local HL = "TypedownRenderLink"

-- Apply underline highlight to col range on row
local function underline(buf, ns, row, col, end_col)
	vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
		end_col = end_col,
		hl_group = HL,
		priority = 110,
	})
end

function M.render(buf)
	local ns = state.get_namespace(M.name)
	state.clear(buf, M.name)

	local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
	for i, line in ipairs(lines) do
		local row = i - 1
		if not state.is_hidden(buf, row) then
			M.render_frefs(buf, ns, row, line)
			M.render_md_links(buf, ns, row, line)
		end
	end
end

-- Underline fref("...") and ${fref("...")} patterns
function M.render_frefs(buf, ns, row, line)
	local search_start = 1
	while true do
		local m_start, m_end = line:find('fref%("([^"]+)"%)', search_start)
		if not m_start then
			break
		end
		underline(buf, ns, row, m_start - 1, m_end)
		search_start = m_end + 1
	end
end

-- Underline [text](url) and ![alt](url)
function M.render_md_links(buf, ns, row, line)
	local search_start = 1
	while true do
		local m_start, m_end = line:find("!?%[.-%]%(.-%)", search_start)
		if not m_start then
			break
		end
		underline(buf, ns, row, m_start - 1, m_end)
		search_start = m_end + 1
	end
end

return M
