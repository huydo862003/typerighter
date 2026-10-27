-- Heading renderer: icon overlay + background + sign column indicator
local state = require("typedown.render.state")

local M = {}
M.name = "headings"

local ICONS = { "󰉫", "󰉬", "󰉭", "󰉮", "󰉯", "󰉰" }
local SIGNS = { "H1", "H2", "H3", "H4", "H5", "H6" }
local HL_FG  = { "TypedownRenderH1", "TypedownRenderH2", "TypedownRenderH3", "TypedownRenderH4", "TypedownRenderH5", "TypedownRenderH6" }
local HL_BG  = { "TypedownRenderH1Bg", "TypedownRenderH2Bg", "TypedownRenderH3Bg", "TypedownRenderH4Bg", "TypedownRenderH5Bg", "TypedownRenderH6Bg" }

local CAPTURE_LEVEL = {
  h1 = 1, h2 = 2, h3 = 3, h4 = 4, h5 = 5, h6 = 6,
}

-- Elements that are always visible (not hidden by anti-conceal)
M.always_visible = { "background", "sign" }

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

  local q = get_query()
  for id, node in q:iter_captures(md_tree:root(), buf) do
    local level = CAPTURE_LEVEL[q.captures[id]]
    if level then
      local start_row, marker_col, _, marker_end_col = node:range()
      local hidden = state.is_hidden(buf, start_row)

      -- Sign column indicator (always visible)
      vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
        sign_text = SIGNS[level],
        sign_hl_group = HL_FG[level],
        priority = 110,
      })

      -- Full-line background (always visible)
      vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
        end_row  = start_row + 1,
        hl_group = HL_BG[level],
        hl_eol   = true,
        priority = 100,
      })

      -- Overlay marker with icon (hidden on cursor line)
      if not hidden then
        local marker_width = marker_end_col - marker_col
        local text = ICONS[level] .. string.rep(" ", math.max(1, marker_width - 1))

        vim.api.nvim_buf_set_extmark(buf, ns, start_row, marker_col, {
          end_col       = marker_end_col,
          virt_text     = { { text, HL_FG[level] } },
          virt_text_pos = "overlay",
          priority      = 110,
        })
      end
    end
  end
end

return M
