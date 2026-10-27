-- Link renderer: conceal fref("path") and ${fref("path")} with icon + name
local state = require("typedown.render.state")

local M = {}
M.name = "links"

local LINK_ICON = " "

-- Extract display name from a fref path: "/people/alice.td" -> "alice"
local function display_name(path)
  local name = path:match("([^/]+)%.td$") or path:match("([^/]+)$") or path
  -- Convert kebab-case/snake_case to title
  return name:gsub("[-_]", " ")
end

function M.render(buf)
  local ns = state.get_namespace(M.name)
  state.clear(buf, M.name)

  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  for i, line in ipairs(lines) do
    local row = i - 1
    if not state.is_hidden(buf, row) then
      -- Match ${fref("...")} in markdown body
      M.render_interpolated_frefs(buf, ns, row, line)
      -- Match fref("...") in frontmatter YAML values
      M.render_yaml_frefs(buf, ns, row, line)
    end
  end
end

-- Render ${fref("path")} patterns (markdown body interpolations)
function M.render_interpolated_frefs(buf, ns, row, line)
  local search_start = 1
  while true do
    local m_start, m_end, path = line:find('%${fref%("([^"]+)"%)}', search_start)
    if not m_start then break end

    local name = display_name(path)
    local col = m_start - 1  -- 0-indexed
    local end_col = m_end    -- 0-indexed exclusive

    vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
      end_col       = end_col,
      virt_text     = {
        { LINK_ICON, "TypedownRenderFrefIcon" },
        { name, "TypedownRenderFrefText" },
      },
      virt_text_pos = "overlay",
      priority      = 110,
    })

    search_start = m_end + 1
  end
end

-- Render fref("path") patterns (YAML frontmatter values)
function M.render_yaml_frefs(buf, ns, row, line)
  local search_start = 1
  while true do
    -- Match standalone fref("...") but NOT ${fref("...")} (already handled above)
    local m_start, m_end, path = line:find('fref%("([^"]+)"%)', search_start)
    if not m_start then break end

    -- Skip if preceded by ${ (already rendered as interpolation)
    if m_start >= 3 and line:sub(m_start - 2, m_start - 1) == "${" then
      search_start = m_end + 1
    else
      local name = display_name(path)
      local col = m_start - 1
      local end_col = m_end

      vim.api.nvim_buf_set_extmark(buf, ns, row, col, {
        end_col       = end_col,
        virt_text     = {
          { LINK_ICON, "TypedownRenderFrefIcon" },
          { name, "TypedownRenderFrefText" },
        },
        virt_text_pos = "overlay",
        priority      = 110,
      })

      search_start = m_end + 1
    end
  end
end

return M
