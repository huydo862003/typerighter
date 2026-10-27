-- Highlight group definitions for the rendering engine
-- All groups use `default = true` so users can override via their colorscheme

local M = {}

function M.setup()
  local hl = vim.api.nvim_set_hl

  -- Headings: icon foreground + full-line background
  hl(0, "TypedownRenderH1", { fg = "#f38ba8", bold = true, default = true })
  hl(0, "TypedownRenderH1Bg", { bg = "#3b2030", default = true })
  hl(0, "TypedownRenderH2", { fg = "#fab387", bold = true, default = true })
  hl(0, "TypedownRenderH2Bg", { bg = "#3b2a20", default = true })
  hl(0, "TypedownRenderH3", { fg = "#f9e2af", bold = true, default = true })
  hl(0, "TypedownRenderH3Bg", { bg = "#3b3520", default = true })
  hl(0, "TypedownRenderH4", { fg = "#a6e3a1", bold = true, default = true })
  hl(0, "TypedownRenderH4Bg", { bg = "#203b25", default = true })
  hl(0, "TypedownRenderH5", { fg = "#89b4fa", bold = true, default = true })
  hl(0, "TypedownRenderH5Bg", { bg = "#20253b", default = true })
  hl(0, "TypedownRenderH6", { fg = "#cba6f7", bold = true, default = true })
  hl(0, "TypedownRenderH6Bg", { bg = "#2a203b", default = true })

  -- Containers
  hl(0, "TypedownRenderContainerNote", { fg = "#89b4fa", default = true })
  hl(0, "TypedownRenderContainerWarning", { fg = "#f9e2af", default = true })
  hl(0, "TypedownRenderContainerTip", { fg = "#a6e3a1", default = true })
  hl(0, "TypedownRenderContainerImportant", { fg = "#cba6f7", default = true })
  hl(0, "TypedownRenderContainerCaution", { fg = "#f38ba8", default = true })
  hl(0, "TypedownRenderContainerDefault", { fg = "#89b4fa", default = true })
  hl(0, "TypedownRenderContainerBorder", { fg = "#585b70", default = true })

  -- Code
  hl(0, "TypedownRenderCodeBlock", { bg = "#1e1e2e", default = true })
  hl(0, "TypedownRenderCodeInline", { bg = "#313244", default = true })
  hl(0, "TypedownRenderCodeLanguage", { fg = "#89b4fa", italic = true, default = true })
  hl(0, "TypedownRenderCodeBorder", { fg = "#585b70", default = true })

  -- Tables
  hl(0, "TypedownRenderTableBorder", { fg = "#585b70", default = true })
  hl(0, "TypedownRenderTableHeader", { bold = true, default = true })

  -- Checkboxes
  hl(0, "TypedownRenderCheckboxUnchecked", { fg = "#585b70", default = true })
  hl(0, "TypedownRenderCheckboxChecked", { fg = "#a6e3a1", default = true })

  -- Bullets
  hl(0, "TypedownRenderBullet", { fg = "#89b4fa", default = true })

  -- Toggle list
  hl(0, "TypedownRenderToggleOpen", { fg = "#89b4fa", default = true })
  hl(0, "TypedownRenderToggleClosed", { fg = "#585b70", default = true })
end

return M
