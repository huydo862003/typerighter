-- Blockquote renderer: grey left gutter bar on every line, conceal > markers
local state = require("typedown.render.state")

local M = {}
M.name = "blockquotes"

local u = utf8 and utf8.char or function(cp)
	if cp < 0x80 then
		return string.char(cp)
	elseif cp < 0x800 then
		return string.char(0xC0 + math.floor(cp / 64), 0x80 + cp % 64)
	elseif cp < 0x10000 then
		return string.char(0xE0 + math.floor(cp / 4096), 0x80 + math.floor(cp / 64) % 64, 0x80 + cp % 64)
	else
		return string.char(
			0xF0 + math.floor(cp / 262144),
			0x80 + math.floor(cp / 4096) % 64,
			0x80 + math.floor(cp / 64) % 64,
			0x80 + cp % 64
		)
	end
end

local BORDER = u(0x258E) -- ▎

local query

local function get_query()
	if not query then
		query = vim.treesitter.query.parse("typedown_md", [[(block_quote) @bq]])
	end
	return query
end

function M.render(buf, md_tree)
	local ns = state.get_namespace(M.name)
	state.clear(buf, M.name)
	local hidden = state.is_hidden

	local q = get_query()
	for _, node in q:iter_captures(md_tree:root(), buf) do
		local start_row, _, end_row, end_col = node:range()
		local last_row = end_col > 0 and end_row or end_row - 1
		local lines = vim.api.nvim_buf_get_lines(buf, start_row, last_row + 1, false)

		for i, line in ipairs(lines) do
			local row = start_row + i - 1
			if not hidden(buf, row) then
				-- Gutter bar
				vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
					virt_text = { { BORDER, "TypedownRenderBlockQuoteBorder" } },
					virt_text_pos = "inline",
					priority = 90,
				})

				-- Conceal leading "> " (with optional trailing space)
				local col = line:find("^> ?")
				if col then
					local end_c = line:match("^> ") and 2 or 1
					vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
						end_col = end_c,
						conceal = "",
						priority = 110,
					})
				end
			end
		end
	end
end

return M
