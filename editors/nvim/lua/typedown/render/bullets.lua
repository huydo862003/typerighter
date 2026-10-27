-- Bullet point renderer: replace list markers with styled icons
local state = require("typedown.render.state")

local M = {}
M.name = "bullets"

local BULLET_ICONS = { "●", "○", "◆", "◇", "■", "□" }

local query

local function get_query()
  if not query then
    query = vim.treesitter.query.parse("typedown_md", [[
      (list_marker_minus) @marker
      (list_marker_star)  @marker
      (list_marker_dot)   @marker
    ]])
  end
  return query
end

-- Count enclosing bullet_list/ordered_list ancestors for nesting depth.
-- Top-level markers sit inside one list node so depth = count - 1.
local function list_depth(node)
  local count = 0
  local parent = node:parent()
  while parent do
    local kind = parent:type()
    if kind == "bullet_list" or kind == "ordered_list" then count = count + 1 end
    parent = parent:parent()
  end
  return math.max(0, count - 1)
end

function M.render(buf, md_tree)
  local ns = state.get_namespace(M.name)
  state.clear(buf, M.name)

  local q = get_query()
  for _, node in q:iter_captures(md_tree:root(), buf) do
    local row, col, _, end_col = node:range()
    if not state.is_hidden(buf, row) then
      local depth = list_depth(node)
      local icon  = BULLET_ICONS[(depth % #BULLET_ICONS) + 1]
      local marker_width = end_col - col
      local text = icon .. string.rep(" ", math.max(0, marker_width - 1))

      vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
        end_col       = end_col,
        virt_text     = { { text, "TypedownRenderBullet" } },
        virt_text_pos = "overlay",
        priority      = 110,
      })
    end
  end
end

return M
