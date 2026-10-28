-- Code block renderer: background + left border + language badge + padding
local state = require("typedown.render.state")

local M = {}
M.name = "code"

local LEFT_PAD = 1 -- cells of padding on the left inside the block
local MIN_WIDTH = 40 -- minimum block width in cells

-- Language icons (Nerd Font devicons, using utf8.char for reliability)
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

local LANG_ICONS = {
	-- Web
	js = u(0xF031E),
	javascript = u(0xF031E),
	jsx = u(0xF031E),
	ts = u(0xF06E6),
	typescript = u(0xF06E6),
	tsx = u(0xF06E6),
	html = u(0xE736),
	css = u(0xE749),
	scss = u(0xE749),
	sass = u(0xE603),
	less = u(0xE758),
	vue = u(0xF0844),
	svelte = u(0xE697),
	astro = u(0xE6B2),

	-- Systems
	c = u(0xE61E),
	cpp = u(0xE61D),
	["c++"] = u(0xE61D),
	h = u(0xE61E),
	hpp = u(0xE61D),
	rs = u(0xE7A8),
	rust = u(0xE7A8),
	go = u(0xE627),
	zig = u(0xF0E7),

	-- Scripting
	py = u(0xE73C),
	python = u(0xE73C),
	rb = u(0xE791),
	ruby = u(0xE791),
	lua = u(0xE620),
	perl = u(0xE769),
	php = u(0xE73D),
	r = u(0xF07D4),

	-- JVM
	java = u(0xE738),
	kotlin = u(0xE634),
	scala = u(0xE737),
	groovy = u(0xE775),
	clojure = u(0xE76A),

	-- .NET
	cs = u(0xF031B),
	["c#"] = u(0xF031B),
	fs = u(0xE7A7),
	["f#"] = u(0xE7A7),

	-- Shell
	sh = u(0xF489),
	bash = u(0xF489),
	zsh = u(0xF489),
	fish = u(0xF489),
	ps1 = u(0xF0A0A),
	powershell = u(0xF0A0A),

	-- Data/Config
	json = u(0xE60B),
	jsonc = u(0xE60B),
	yaml = u(0xE6A8),
	yml = u(0xE6A8),
	toml = u(0xE6B2),
	xml = u(0xF05C0),
	csv = u(0xF0219),
	ini = u(0xE615),

	-- Markup/Docs
	markdown = u(0xE73E),
	md = u(0xE73E),
	mdx = u(0xE73E),
	tex = u(0xE69B),
	latex = u(0xE69B),

	-- Diagrams
	d2 = u(0xF021C),
	mermaid = u(0xF021C),

	-- Database
	sql = u(0xE706),
	graphql = u(0xF0847),
	prisma = u(0xE684),

	-- DevOps
	dockerfile = u(0xF0868),
	docker = u(0xF0868),
	terraform = u(0xF1062),
	tf = u(0xF1062),
	nix = u(0xF1105),

	-- Mobile
	swift = u(0xE755),
	dart = u(0xE798),

	-- Functional
	haskell = u(0xE777),
	hs = u(0xE777),
	elixir = u(0xE62D),
	ex = u(0xE62D),
	erlang = u(0xE7B1),
	ocaml = u(0xE67A),
	elm = u(0xE62C),

	-- Other
	vim = u(0xE62B),
	diff = u(0xE728),
	git = u(0xE702),
	makefile = u(0xE779),
	cmake = u(0xE615),
	proto = u(0xE60E),
	wasm = u(0xE6A1),
	txt = u(0xF0219),
	log = u(0xF0219),
}

local query

local function get_query()
	if not query then
		query = vim.treesitter.query.parse(
			"typedown_md",
			[[
      (fenced_code_block) @code_block
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
		M.render_block(buf, ns, node)
	end
end

function M.render_block(buf, ns, node)
	local start_row, _, end_row = node:range()
	local hidden = state.is_hidden

	-- Calculate max content width for the block
	local max_width = 0
	for row = start_row, end_row - 1 do
		local line = vim.api.nvim_buf_get_lines(buf, row, row + 1, false)[1] or ""
		max_width = math.max(max_width, vim.fn.strdisplaywidth(line))
	end
	local block_width = math.max(max_width + LEFT_PAD * 2, MIN_WIDTH)

	for row = start_row, end_row - 1 do
		-- Background with hl_eol (always visible)
		vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
			end_row = row + 1,
			hl_group = "TypedownRenderCodeBlock",
			hl_eol = true,
			priority = 80,
		})

		-- Right-side padding to extend background to block_width
		local line = vim.api.nvim_buf_get_lines(buf, row, row + 1, false)[1] or ""
		local line_width = vim.fn.strdisplaywidth(line)
		local right_pad = math.max(0, block_width - line_width)
		if right_pad > 0 then
			vim.api.nvim_buf_set_extmark(buf, ns, row, #line, {
				virt_text = { { string.rep(" ", right_pad), "TypedownRenderCodeBlock" } },
				virt_text_pos = "inline",
				priority = 80,
			})
		end

		-- Left border (hidden on cursor line)
		if not hidden(buf, row) then
			vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
				virt_text = { { u(0x258E), "TypedownRenderCodeBorder" } },
				virt_text_pos = "inline",
				priority = 90,
			})
		end
	end

	-- Language badge on opening fence line (hidden on cursor line)
	if not hidden(buf, start_row) then
		local lang_nodes = node:field("language")
		if lang_nodes and lang_nodes[1] then
			local lang = vim.treesitter.get_node_text(lang_nodes[1], buf):lower()
			-- Strip line range indicator like {1,3}
			lang = lang:gsub("{.*", "")
			local icon = LANG_ICONS[lang]
			local badge = icon and (icon .. " " .. lang) or lang
			if badge ~= "" then
				vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
					virt_text = { { " " .. badge .. " ", "TypedownRenderCodeLanguage" } },
					virt_text_pos = "eol",
					priority = 100,
				})
			end
		end
	end
end

return M
