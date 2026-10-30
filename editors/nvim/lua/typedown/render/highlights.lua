-- Highlight group definitions for the rendering engine
-- Colors based on Catppuccin Mocha palette
-- All groups use `default = true` so users can override via their colorscheme

local M = {}

function M.setup()
	local set_hl = vim.api.nvim_set_hl

	-- Headings: icon (fg+bg combined) + full-line background
	set_hl(0, "TypedownRenderH1", { fg = "#f38ba8", bg = "#3b2030", bold = true, default = true })
	set_hl(0, "TypedownRenderH1Bg", { bg = "#3b2030", default = true })
	set_hl(0, "TypedownRenderH2", { fg = "#fab387", bg = "#3b2a20", bold = true, default = true })
	set_hl(0, "TypedownRenderH2Bg", { bg = "#3b2a20", default = true })
	set_hl(0, "TypedownRenderH3", { fg = "#f9e2af", bg = "#3b3520", bold = true, default = true })
	set_hl(0, "TypedownRenderH3Bg", { bg = "#3b3520", default = true })
	set_hl(0, "TypedownRenderH4", { fg = "#a6e3a1", bg = "#203b25", bold = true, default = true })
	set_hl(0, "TypedownRenderH4Bg", { bg = "#203b25", default = true })
	set_hl(0, "TypedownRenderH5", { fg = "#89b4fa", bg = "#20253b", bold = true, default = true })
	set_hl(0, "TypedownRenderH5Bg", { bg = "#20253b", default = true })
	set_hl(0, "TypedownRenderH6", { fg = "#cba6f7", bg = "#2a203b", bold = true, default = true })
	set_hl(0, "TypedownRenderH6Bg", { bg = "#2a203b", default = true })

	-- Containers
	set_hl(0, "TypedownRenderContainerNote", { fg = "#89b4fa", default = true })
	set_hl(0, "TypedownRenderContainerWarning", { fg = "#f9e2af", default = true })
	set_hl(0, "TypedownRenderContainerTip", { fg = "#a6e3a1", default = true })
	set_hl(0, "TypedownRenderContainerImportant", { fg = "#cba6f7", default = true })
	set_hl(0, "TypedownRenderContainerCaution", { fg = "#f38ba8", default = true })
	set_hl(0, "TypedownRenderContainerDefault", { fg = "#89b4fa", default = true })

	-- Code
	set_hl(0, "TypedownRenderCodeBlock", { bg = "#1e1e2e", default = true })
	set_hl(0, "TypedownRenderCodeLanguage", { fg = "#89b4fa", italic = true, default = true })
	set_hl(0, "TypedownRenderCodeBorder", { fg = "#585b70", default = true })

	-- Tables
	set_hl(0, "TypedownRenderTableBorder", { fg = "#585b70", default = true })
	set_hl(0, "TypedownRenderTableHeader", { bold = true, default = true })

	-- Checkboxes (multi-state)
	set_hl(0, "TypedownRenderCheckboxUnchecked", { fg = "#585b70", default = true })
	set_hl(0, "TypedownRenderCheckboxChecked", { fg = "#a6e3a1", default = true })
	set_hl(0, "TypedownRenderCheckboxCancelled", { fg = "#f38ba8", strikethrough = true, default = true })
	set_hl(0, "TypedownRenderCheckboxForwarded", { fg = "#fab387", default = true })
	set_hl(0, "TypedownRenderCheckboxImportant", { fg = "#f38ba8", bold = true, default = true })

	-- Links (fref, markdown links, media embeds)
	set_hl(0, "TypedownRenderLink", { fg = "#89b4fa", underline = true, default = true })

	-- Bullets
	set_hl(0, "TypedownRenderBullet", { fg = "#89b4fa", default = true })

	-- Toggle list
	set_hl(0, "TypedownRenderToggleOpen", { fg = "#89b4fa", default = true })
	set_hl(0, "TypedownRenderToggleClosed", { fg = "#585b70", default = true })

	-- Frontmatter
	set_hl(0, "TypedownRenderFrontmatterRule", { fg = "#585b70", default = true })
	set_hl(0, "TypedownRenderFrontmatterContent", { fg = "#7f849c", default = true })

	-- Inline formatting (ensures correct rendering regardless of colorscheme)
	set_hl(0, "@markup.strong", { bold = true, default = true })
	set_hl(0, "@markup.italic", { italic = true, default = true })
	set_hl(0, "@markup.strikethrough", { strikethrough = true, default = true })
end

return M
