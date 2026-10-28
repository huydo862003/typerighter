-- Frontmatter renderer: replace --- delimiters with thin horizontal rules
local state = require("typedown.render.state")

local M = {}
M.name = "frontmatter"

function M.render(buf)
	local ns = state.get_namespace(M.name)
	state.clear(buf, M.name)

	local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
	local open_row, close_row

	-- Find frontmatter delimiters (first two --- lines)
	for i, line in ipairs(lines) do
		if line:match("^%-%-%-$") then
			if not open_row then
				open_row = i - 1
			else
				close_row = i - 1
				break
			end
		end
	end

	if not open_row or not close_row then
		return
	end

	local win_width = vim.api.nvim_win_get_width(0)

	for _, row in ipairs({ open_row, close_row }) do
		if not state.is_hidden(buf, row) then
			local rule = string.rep("─", win_width)
			vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
				end_col = 3,
				virt_text = { { rule, "TypedownRenderFrontmatterRule" } },
				virt_text_pos = "overlay",
				priority = 110,
			})
		end
	end

	-- Dim frontmatter content lines
	for row = open_row + 1, close_row - 1 do
		vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
			end_row = row + 1,
			hl_group = "TypedownRenderFrontmatterContent",
			hl_eol = true,
			priority = 80,
		})
	end
end

return M
