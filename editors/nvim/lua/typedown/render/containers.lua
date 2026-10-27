-- Container block renderer: icon + colored left border for ::: blocks
local state = require("typedown.render.state")

local M = {}
M.name = "containers"

local CONTAINER_STYLES = {
  note      = { icon = "󰋽", hl = "TypedownRenderContainerNote" },
  info      = { icon = "󰋽", hl = "TypedownRenderContainerNote" },
  tip       = { icon = "󰌶", hl = "TypedownRenderContainerTip" },
  warning   = { icon = "󰀪", hl = "TypedownRenderContainerWarning" },
  caution   = { icon = "󱧡", hl = "TypedownRenderContainerCaution" },
  danger    = { icon = "󱧡", hl = "TypedownRenderContainerCaution" },
  important = { icon = "󰅾", hl = "TypedownRenderContainerImportant" },
  card      = { icon = "󰾵", hl = "TypedownRenderContainerDefault" },
  details   = { icon = "󰁅", hl = "TypedownRenderContainerDefault" },
  tabs      = { icon = "󰓩", hl = "TypedownRenderContainerDefault" },
}

local DEFAULT_STYLE = { icon = "󰅩", hl = "TypedownRenderContainerDefault" }

local query

local function get_query()
  if not query then
    query = vim.treesitter.query.parse("typedown_md", [[
      (container_block) @container
    ]])
  end
  return query
end

function M.render(buf, md_tree)
  local ns = state.get_namespace(M.name)
  state.clear(buf, M.name)
  local cursor_line = state.get_cursor_line(buf)

  local q = get_query()
  for _, node in q:iter_captures(md_tree:root(), buf) do
    M.render_container(buf, ns, node, cursor_line)
  end
end

function M.render_container(buf, ns, node, cursor_line)
  local start_row, _, end_row = node:range()

  -- Find container type via field
  local type_node = node:field("type")[1]
  local container_type = type_node and vim.treesitter.get_node_text(type_node, buf) or ""
  local style = CONTAINER_STYLES[container_type] or DEFAULT_STYLE

  -- Left border on each line
  for row = start_row, end_row - 1 do
    if row ~= cursor_line then
      vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
        virt_text     = { { "┃ ", style.hl } },
        virt_text_pos = "inline",
        priority      = 90,
      })
    end
  end

  -- Conceal/replace ::: delimiters
  for child in node:iter_children() do
    if child:type() == "container_block_delimiter" then
      local d_row, d_col, _, d_end_col = child:range()
      if d_row == start_row and d_row ~= cursor_line then
        -- Opening ::: -> icon
        vim.api.nvim_buf_set_extmark(buf, ns, d_row, d_col, {
          end_col = d_end_col,
          conceal = "",
        })
        vim.api.nvim_buf_set_extmark(buf, ns, d_row, d_col, {
          virt_text     = { { style.icon .. " ", style.hl } },
          virt_text_pos = "inline",
          priority      = 110,
        })
      elseif d_row == end_row - 1 and d_row ~= cursor_line then
        -- Closing ::: -> conceal
        vim.api.nvim_buf_set_extmark(buf, ns, d_row, d_col, {
          end_col = d_end_col,
          conceal = "",
        })
      end
    end
  end
end

return M
