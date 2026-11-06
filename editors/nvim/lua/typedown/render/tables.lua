-- Table renderer: conceal pipes and delimiter chars, replace with box-drawing
local state = require("typedown.render.state")

local M = {}
M.name = "tables"

local BORDER = {
	horizontal = "─",
	vertical = "│",
}

local table_query
local pipe_query

local function get_table_query()
	if not table_query then
		table_query = vim.treesitter.query.parse(
			"typedown_md",
			[[
      (pipe_table) @table
    ]]
		)
	end
	return table_query
end

-- Query for pipe separator tokens that are direct children of table rows
local function get_pipe_query()
	if not pipe_query then
		pipe_query = vim.treesitter.query.parse(
			"typedown_md",
			[[
      (pipe_table_row "|" @pipe)
      (pipe_table_header "|" @pipe)
      (pipe_table_delimiter_row "|" @pipe)
    ]]
		)
	end
	return pipe_query
end

function M.render(buf, md_tree)
	local ns = state.get_namespace(M.name)
	state.clear(buf, M.name)

	local tq = get_table_query()
	for _, node in tq:iter_captures(md_tree:root(), buf) do
		M.render_table(buf, ns, node, md_tree)
	end
end

function M.render_table(buf, ns, node, md_tree)
	local start_row, _, end_row = node:range()

	-- Ensure conceallevel is set so conceal extmarks work
	for _, win in ipairs(vim.fn.win_findbuf(buf)) do
		if vim.api.nvim_get_option_value("conceallevel", { win = win }) < 2 then
			vim.api.nvim_set_option_value("conceallevel", 2, { win = win })
		end
	end

	-- Use tree-sitter to find actual pipe separators, skipping | inside cell content
	local pq = get_pipe_query()
	for _, pipe_node in pq:iter_captures(md_tree:root(), buf, start_row, end_row) do
		local row, col = pipe_node:range()
		if not state.is_hidden(buf, row) then
			vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
				end_col = col + 1,
				conceal = "",
			})
			vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
				virt_text = { { BORDER.vertical, "TypedownRenderTableBorder" } },
				virt_text_pos = "inline",
				priority = 110,
			})
		end
	end

	-- Replace delimiter row - and : with horizontal borders
	for row = start_row, end_row - 1 do
		if not state.is_hidden(buf, row) then
			local line = vim.api.nvim_buf_get_lines(buf, row, row + 1, false)[1] or ""
			local is_delimiter = line:match("^%s*[|%-%s:]+$") ~= nil

			if is_delimiter then
				for col_index = 1, #line do
					local char = line:sub(col_index, col_index)
					if char == "-" or char == ":" then
						vim.api.nvim_buf_set_extmark(buf, ns, row, col_index - 1, {
							end_col = col_index,
							conceal = "",
						})
						vim.api.nvim_buf_set_extmark(buf, ns, row, col_index - 1, {
							virt_text = { { BORDER.horizontal, "TypedownRenderTableBorder" } },
							virt_text_pos = "inline",
							priority = 110,
						})
					end
				end
			end

			-- Header row bold
			if row == start_row then
				vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
					end_row = row + 1,
					hl_group = "TypedownRenderTableHeader",
					hl_eol = true,
					priority = 80,
				})
			end
		end
	end
end

return M
