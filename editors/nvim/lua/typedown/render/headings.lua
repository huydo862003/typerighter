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

-- Lazy-initialized query (parser may not be registered at require time)
local query

local function get_query()
  if not query then
    query = vim.treesitter.query.parse("typedown_md", [[
      (atx_heading (atx_h1_marker) @h1)
      (atx_heading (atx_h2_marker) @h2)
      (atx_heading (atx_h3_marker) @h3)
      (atx_heading (atx_h4_marker) @h4)
      (atx_heading (atx_h5_marker) @h5)
      (atx_heading (atx_h6_marker) @h6)
    ]])
  end
  return query
end

function M.render(buf, md_tree)
  local ns = state.get_namespace(M.name)
  state.clear(buf, M.name)
  local cursor_line = state.get_cursor_line(buf)

  local q = get_query()
  for id, node in q:iter_captures(md_tree:root(), buf) do
    local level = CAPTURE_LEVEL[q.captures[id]]
    if level then
      local start_row, marker_col, _, marker_end_col = node:range()
      if start_row ~= cursor_line then
        -- Conceal the marker and trailing space
        vim.api.nvim_buf_set_extmark(buf, ns, start_row, marker_col, {
          end_col = marker_end_col + 1,
          conceal = "",
        })

        -- Icon in place of marker
        vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
          virt_text     = { { ICONS[level] .. " ", HL_FG[level] } },
          virt_text_pos = "inline",
          priority      = 110,
        })

        -- Full-line background
        vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
          end_row  = start_row + 1,
          hl_group = HL_BG[level],
          hl_eol   = true,
          priority = 100,
        })
      end
    end
  end
end

return M
