-- Main rendering engine: attach/detach per buffer, orchestrate renderers

local state = require("typedown.render.state")
local highlights = require("typedown.render.highlights")
local anti_conceal = require("typedown.render.anti-conceal")
local headings = require("typedown.render.headings")
local containers = require("typedown.render.containers")
local checkboxes = require("typedown.render.checkboxes")
local bullets = require("typedown.render.bullets")
local code = require("typedown.render.code")
local blockquotes = require("typedown.render.blockquotes")
local tables = require("typedown.render.tables")
local toggle_list = require("typedown.render.toggle-list")
local links = require("typedown.render.links")
local frontmatter = require("typedown.render.frontmatter")
local smart_action = require("typedown.render.smart-action")

local M = {}

local renderers = {
	headings,
	containers,
	checkboxes,
	bullets,
	code,
	blockquotes,
	tables,
	toggle_list,
}

-- Links renderer runs separately (not tree-sitter based, scans full buffer)
local text_renderers = {
	links,
	frontmatter,
}

function M.setup()
	highlights.setup()
	checkboxes.setup_commands()
	toggle_list.setup_commands()
end

function M.render_buf(buf)
	-- If mode is disabled (e.g. insert), clear everything instead of rendering
	if state.is_mode_disabled(buf) then
		for _, renderer in ipairs(renderers) do
			state.clear(buf, renderer.name)
		end
		for _, renderer in ipairs(text_renderers) do
			state.clear(buf, renderer.name)
		end
		return
	end

	-- Max file length guard
	local line_count = vim.api.nvim_buf_line_count(buf)
	if line_count > state.max_file_length then
		return
	end

	local parser = vim.treesitter.get_parser(buf, "typedown")
	if not parser then
		return
	end

	parser:parse()

	-- Find the typedown_md subtree (injected from the body)
	for _, child_tree in pairs(parser:children()) do
		if child_tree:lang() == "typedown_md" then
			for _, tree in ipairs(child_tree:trees()) do
				for _, renderer in ipairs(renderers) do
					local ok, err = pcall(renderer.render, buf, tree)
					if not ok then
						vim.schedule(function()
							vim.notify("[typedown render] " .. tostring(err), vim.log.levels.DEBUG)
						end)
					end
				end
			end
			break
		end
	end

	-- Text-based renderers (scan full buffer, not tree-sitter)
	for _, renderer in ipairs(text_renderers) do
		local ok, err = pcall(renderer.render, buf)
		if not ok then
			vim.schedule(function()
				vim.notify("[typedown render] " .. tostring(err), vim.log.levels.DEBUG)
			end)
		end
	end
end

local function set_fold_options(win)
	vim.api.nvim_set_option_value("foldmethod", "expr", { win = win })
	vim.api.nvim_set_option_value("foldexpr", "v:lua.vim.treesitter.foldexpr()", { win = win })
	vim.api.nvim_set_option_value("foldlevel", 99, { win = win })
end

function M.attach(buf)
	if state.is_attached(buf) then
		return
	end
	state.mark_attached(buf)

	-- Smart action keybind (<CR> = follow link / toggle checkbox / fold)
	smart_action.setup(buf)

	-- Enable tree-sitter folding (za/zo/zc work on containers, code blocks, etc.)
	for _, win in ipairs(vim.fn.win_findbuf(buf)) do
		set_fold_options(win)
	end

	-- Initial cursor position
	local ok, cursor = pcall(vim.api.nvim_win_get_cursor, 0)
	if ok then
		state.set_cursor_line(buf, cursor[1] - 1)
	end

	-- Initial render (deferred to let tree-sitter parse first)
	vim.defer_fn(function()
		if vim.api.nvim_buf_is_valid(buf) then
			M.render_buf(buf)
		end
	end, 100)

	-- Debounced re-render on text changes
	local render_timer = vim.uv.new_timer()
	vim.api.nvim_buf_attach(buf, false, {
		on_lines = function()
			render_timer:stop()
			render_timer:start(
				state.debounce_ms,
				0,
				vim.schedule_wrap(function()
					if vim.api.nvim_buf_is_valid(buf) and state.has_changed(buf) then
						M.render_buf(buf)
					end
				end)
			)
		end,
		on_detach = function()
			state.detach(buf)
		end,
	})

	local group = vim.api.nvim_create_augroup("typedown_render_" .. buf, { clear = true })

	-- Anti-conceal: re-render on cursor move
	vim.api.nvim_create_autocmd("CursorMoved", {
		group = group,
		buffer = buf,
		callback = function()
			if anti_conceal.on_cursor_moved(buf) then
				M.render_buf(buf)
			end
		end,
	})

	-- Mode-aware rendering: re-render on mode change
	vim.api.nvim_create_autocmd("ModeChanged", {
		group = group,
		buffer = buf,
		callback = function()
			if anti_conceal.on_mode_changed(buf) then
				M.render_buf(buf)
			end
		end,
	})

	vim.api.nvim_create_autocmd("InsertEnter", {
		group = group,
		buffer = buf,
		callback = function()
			if anti_conceal.on_insert_enter(buf) then
				M.render_buf(buf) -- will clear due to mode check
			end
		end,
	})

	vim.api.nvim_create_autocmd("InsertLeave", {
		group = group,
		buffer = buf,
		callback = function()
			if anti_conceal.on_insert_leave(buf) then
				M.render_buf(buf)
			end
		end,
	})

	vim.api.nvim_create_autocmd("BufWinEnter", {
		group = group,
		buffer = buf,
		callback = function()
			set_fold_options(vim.api.nvim_get_current_win())
		end,
	})

	vim.api.nvim_create_autocmd("BufDelete", {
		group = group,
		buffer = buf,
		callback = function()
			state.detach(buf)
			pcall(vim.api.nvim_del_augroup_by_id, group)
		end,
	})
end

return M
