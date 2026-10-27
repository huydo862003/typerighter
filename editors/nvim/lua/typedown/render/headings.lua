-- Heading renderer: icon + background per level, conceal # markers
local state = require("typedown.render.state")

local M = {}

local ICONS = { "󰉫", "󰉬", "󰉭", "󰉮", "󰉯", "󰉰" }
local HL_FG = { "TypedownRenderH1", "TypedownRenderH2", "TypedownRenderH3", "TypedownRenderH4", "TypedownRenderH5", "TypedownRenderH6" }
local HL_BG = { "TypedownRenderH1Bg", "TypedownRenderH2Bg", "TypedownRenderH3Bg", "TypedownRenderH4Bg", "TypedownRenderH5Bg", "TypedownRenderH6Bg" }

local MARKER_KINDS = {
  atx_h1_marker = 1,
  atx_h2_marker = 2,
  atx_h3_marker = 3,
  atx_h4_marker = 4,
  atx_h5_marker = 5,
  atx_h6_marker = 6,
}

function M.render(buf, md_tree)
  local ns = state.get_namespace("headings")
  state.clear(buf, "headings")
  local cursor_line = state.get_cursor_line(buf)

  local query = vim.treesitter.query.get("typedown_md", "highlights")
  if not query then return end

  for id, node in query:iter_captures(md_tree:root(), buf) do
    local name = query.captures[id]
    local level = MARKER_KINDS[name]
    if level then
      local heading = node:parent()
      if not heading then goto continue end

      local start_row, start_col, end_row = heading:range()

      -- Anti-conceal: skip cursor line
      if start_row == cursor_line then goto continue end

      local icon = ICONS[level] or "#"
      local hl_fg = HL_FG[level] or HL_FG[1]
      local hl_bg = HL_BG[level] or HL_BG[1]

      -- Overlay the marker with the icon
      local marker_start, marker_col, _, marker_end_col = node:range()
      vim.api.nvim_buf_set_extmark(buf, ns, marker_start, marker_col, {
        end_col = marker_end_col + 1, -- +1 for the space after #
        conceal = "",
      })

      -- Virtual text with icon at the start
      vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
        virt_text = { { icon .. " ", hl_fg } },
        virt_text_pos = "inline",
        priority = 110,
      })

      -- Background highlight for the full line
      vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
        end_row = start_row + 1,
        hl_group = hl_bg,
        hl_eol = true,
        priority = 100,
      })

      ::continue::
    end
  end
end

return M
