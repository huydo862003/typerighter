-- Heading renderer: icon + background per level, conceal # markers
local state = require("typedown.render.state")

local M = {}
M.name = "headings"

local ICONS = { "󰉫", "󰉬", "󰉭", "󰉮", "󰉯", "󰉰" }
local HL_FG  = { "TypedownRenderH1", "TypedownRenderH2", "TypedownRenderH3", "TypedownRenderH4", "TypedownRenderH5", "TypedownRenderH6" }
local HL_BG  = { "TypedownRenderH1Bg", "TypedownRenderH2Bg", "TypedownRenderH3Bg", "TypedownRenderH4Bg", "TypedownRenderH5Bg", "TypedownRenderH6Bg" }

local CAPTURE_LEVEL = {
  h1 = 1, h2 = 2, h3 = 3, h4 = 4, h5 = 5, h6 = 6,
}

-- Compiled once; captures each heading marker with its level in the name
local QUERY = vim.treesitter.query.parse("typedown_md", [[
  (atx_heading (atx_h1_marker) @h1)
  (atx_heading (atx_h2_marker) @h2)
  (atx_heading (atx_h3_marker) @h3)
  (atx_heading (atx_h4_marker) @h4)
  (atx_heading (atx_h5_marker) @h5)
  (atx_heading (atx_h6_marker) @h6)
]])

function M.render(buf, md_tree)
  local ns = state.get_namespace(M.name)
  state.clear(buf, M.name)
  local cursor_line = state.get_cursor_line(buf)

  for id, node in QUERY:iter_captures(md_tree:root(), buf) do
    local level = CAPTURE_LEVEL[QUERY.captures[id]]
    if not level then goto continue end

    local start_row, marker_col, _, marker_end_col = node:range()
    if start_row == cursor_line then goto continue end

    local icon  = ICONS[level]
    local hl_fg = HL_FG[level]
    local hl_bg = HL_BG[level]

    -- Conceal the marker and trailing space
    vim.api.nvim_buf_set_extmark(buf, ns, start_row, marker_col, {
      end_col = marker_end_col + 1, -- +1 for space after #
      conceal = "",
    })

    -- Icon in place of marker
    vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
      virt_text     = { { icon .. " ", hl_fg } },
      virt_text_pos = "inline",
      priority      = 110,
    })

    -- Full-line background
    vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
      end_row  = start_row + 1,
      hl_group = hl_bg,
      hl_eol   = true,
      priority = 100,
    })

    ::continue::
  end
end

return M
