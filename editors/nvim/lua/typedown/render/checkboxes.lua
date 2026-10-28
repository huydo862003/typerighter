-- Checkbox renderer: multi-state icon replacement + cycle toggle command
local state = require("typedown.render.state")

local M = {}
M.name = "checkboxes"

-- Checkbox states in cycle order
-- Each key is the char inside [ ]
-- Toggle cycles: [ ] -> [x] -> [~] -> [>] -> [!] -> [ ]
M.states = {
	[" "] = { icon = "󰄱", hl = "TypedownRenderCheckboxUnchecked", order = 1 },
	["x"] = { icon = "󰄲", hl = "TypedownRenderCheckboxChecked", order = 2 },
	["X"] = { icon = "󰄲", hl = "TypedownRenderCheckboxChecked", order = 2 },
	["~"] = { icon = "󰰱", hl = "TypedownRenderCheckboxCancelled", order = 3 },
	[">"] = { icon = "→", hl = "TypedownRenderCheckboxForwarded", order = 4 },
	["!"] = { icon = "", hl = "TypedownRenderCheckboxImportant", order = 5 },
}

-- Ordered list of state chars for cycling
M.cycle = { " ", "x", "~", ">", "!" }

local query

local function get_query()
	if not query then
		query = vim.treesitter.query.parse(
			"typedown_md",
			[[
      (list_item (checkbox_unchecked) @unchecked)
      (list_item (checkbox_checked)   @checked)
    ]]
		)
	end
	return query
end

function M.render(buf, md_tree)
	local ns = state.get_namespace(M.name)
	state.clear(buf, M.name)

	local q = get_query()
	for _, node in q:iter_captures(md_tree:root(), buf) do
		local row, col, _, end_col = node:range()
		if not state.is_hidden(buf, row) then
			-- Get the actual checkbox text to determine state
			local text = vim.treesitter.get_node_text(node, buf)
			local char = text:match("%[(.-)%]")
			local spec = char and M.states[char] or M.states[" "]

			local node_text = vim.treesitter.get_node_text(node, buf)
			local width = vim.fn.strdisplaywidth(node_text)
			local icon_width = vim.fn.strdisplaywidth(spec.icon)
			local display = spec.icon .. string.rep(" ", math.max(0, width - icon_width))

			vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
				end_col = end_col,
				virt_text = { { display, spec.hl } },
				virt_text_pos = "overlay",
				priority = 110,
			})
		end
	end

	-- Also render checkboxes that tree-sitter may not capture (custom states ~, >, !)
	-- These are in the YAML frontmatter body as raw text patterns
	M.render_custom_states(buf, ns)
end

-- Render custom checkbox states that tree-sitter doesn't capture as checkbox nodes
function M.render_custom_states(buf, ns)
	local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
	for i, line in ipairs(lines) do
		local row = i - 1
		if not state.is_hidden(buf, row) then
			-- Match "- [~] ", "- [>] ", "- [!] " patterns
			local indent, char = line:match("^(%s*)%- %[([~>!])%] ")
			if char and M.states[char] then
				local col = #indent + 2 -- position of [
				local end_col = col + 3 -- position after ]
				local spec = M.states[char]
				local display = spec.icon .. " "

				vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
					end_col = end_col,
					virt_text = { { display, spec.hl } },
					virt_text_pos = "overlay",
					priority = 110,
				})
			end
		end
	end
end

-- Toggle checkbox: cycle through states
function M.toggle()
	local line_nr = vim.api.nvim_win_get_cursor(0)[1]
	local line = vim.api.nvim_buf_get_lines(0, line_nr - 1, line_nr, false)[1]
	if not line then
		return
	end

	local prefix, char, suffix = line:match("^(.-%[)(.)(%].*)$")
	if not char then
		-- Not a checkbox line - convert plain list item to checkbox
		local new_line = line:gsub("^(%s*)([-*+]) (.*)", "%1%2 [ ] %3", 1)
		if new_line ~= line then
			vim.api.nvim_buf_set_lines(0, line_nr - 1, line_nr, false, { new_line })
		end
		return
	end

	-- Find current position in cycle and advance
	local current_idx = 1
	for i, c in ipairs(M.cycle) do
		if c == char or (char == "X" and c == "x") then
			current_idx = i
			break
		end
	end
	local next_idx = (current_idx % #M.cycle) + 1
	local next_char = M.cycle[next_idx]

	local new_line = prefix .. next_char .. suffix
	vim.api.nvim_buf_set_lines(0, line_nr - 1, line_nr, false, { new_line })
end

function M.setup_commands()
	vim.api.nvim_create_user_command("TypedownToggleCheckbox", M.toggle, {
		desc = "Cycle checkbox state on current line",
	})
end

return M
