-- Toggle list renderer: fold/unfold behavior for toggle list items
local state = require("typedown.render.state")

local M = {}
M.name = "toggle_list"

local OPEN_ICON   = ""
local CLOSED_ICON = ""

-- Track folded toggle items: buf -> { [start_row] = true }
local folded = {}

local query, find_query

local function get_query()
  if not query then
    query = vim.treesitter.query.parse("typedown_md", [[
      (toggle_list_item
        (toggle_list_marker) @marker) @item
    ]])
  end
  return query
end

local function get_find_query()
  if not find_query then
    find_query = vim.treesitter.query.parse("typedown_md", [[
      (toggle_list_item) @item
    ]])
  end
  return find_query
end

function M.render(buf, md_tree)
  local ns = state.get_namespace(M.name)
  state.clear(buf, M.name)
  local cursor_line = state.get_cursor_line(buf)

  if not folded[buf] then folded[buf] = {} end

  local q = get_query()
  for id, node in q:iter_captures(md_tree:root(), buf) do
    if q.captures[id] == "item" then
      M.render_item(buf, ns, node, cursor_line)
    end
  end
end

function M.render_item(buf, ns, node, cursor_line)
  local start_row, _, end_row = node:range()
  if start_row == cursor_line then return end

  local is_folded = folded[buf] and folded[buf][start_row]
  local icon = is_folded and CLOSED_ICON or OPEN_ICON
  local hl   = is_folded and "TypedownRenderToggleClosed" or "TypedownRenderToggleOpen"

  -- Find the toggle marker (>)
  for child in node:iter_children() do
    if child:type() == "toggle_list_marker" then
      local m_row, m_col, _, m_end_col = child:range()
      vim.api.nvim_buf_set_extmark(buf, ns, m_row, m_col, {
        end_col = m_end_col,
        conceal = "",
      })
      vim.api.nvim_buf_set_extmark(buf, ns, m_row, m_col, {
        virt_text     = { { icon, hl } },
        virt_text_pos = "inline",
        priority      = 110,
      })
      break
    end
  end

  -- If folded, hide content lines
  if is_folded and end_row - start_row > 1 then
    for row = start_row + 1, end_row - 1 do
      vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
        end_row  = row + 1,
        conceal  = "",
        hl_group = "TypedownRenderToggleClosed",
      })
    end
    vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
      virt_text     = { { " [" .. (end_row - start_row - 1) .. " lines]", "TypedownRenderToggleClosed" } },
      virt_text_pos = "eol",
      priority      = 100,
    })
  end
end

-- Toggle fold state for the toggle list item under cursor
function M.toggle()
  local buf = vim.api.nvim_get_current_buf()
  local line = vim.api.nvim_win_get_cursor(0)[1] - 1

  if not folded[buf] then folded[buf] = {} end

  local fq = get_find_query()
  for _, tree in ipairs(state.get_md_trees(buf)) do
    for id, node in fq:iter_captures(tree:root(), buf) do
      if fq.captures[id] == "item" then
        local start_row, _, end_row = node:range()
        if line >= start_row and line < end_row then
          folded[buf][start_row] = not folded[buf][start_row]
          require("typedown.render").render_buf(buf)
          return
        end
      end
    end
  end
end

function M.setup_commands()
  vim.api.nvim_create_user_command("TypedownToggle", M.toggle, {
    desc = "Toggle fold on toggle list item under cursor",
  })
end

return M
