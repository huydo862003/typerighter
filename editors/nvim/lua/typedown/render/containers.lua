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

  local q = get_query()
  for _, node in q:iter_captures(md_tree:root(), buf) do
    M.render_container(buf, ns, node)
  end
end

function M.render_container(buf, ns, node)
  local start_row, _, end_row = node:range()
  local hidden = state.is_hidden

  -- Find container type via field
  local type_node = node:field("type")[1]
  local container_type = type_node and vim.treesitter.get_node_text(type_node, buf) or ""
  local style = CONTAINER_STYLES[container_type] or DEFAULT_STYLE

  -- Left border on each line (always visible for visual continuity)
  for row = start_row, end_row - 1 do
    vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
      virt_text     = { { "┃ ", style.hl } },
      virt_text_pos = "inline",
      virt_text_repeat_linebreak = true,
      priority      = 90,
    })
  end

  -- Overlay ::: delimiters (hidden on cursor line)
  for child in node:iter_children() do
    if child:type() == "container_block_delimiter" then
      local d_row, d_col, _, d_end_col = child:range()
      if not hidden(buf, d_row) then
        local delim_text = vim.treesitter.get_node_text(child, buf)
        local width = vim.fn.strdisplaywidth(delim_text)
        if d_row == start_row then
          -- Opening ::: -> overlay with icon
          local icon_width = vim.fn.strdisplaywidth(style.icon)
          local text = style.icon .. string.rep(" ", math.max(0, width - icon_width))
          vim.api.nvim_buf_set_extmark(buf, ns, d_row, d_col, {
            end_col       = d_end_col,
            virt_text     = { { text, style.hl } },
            virt_text_pos = "overlay",
            priority      = 110,
          })
        else
          -- Closing ::: -> overlay with spaces (hide it)
          vim.api.nvim_buf_set_extmark(buf, ns, d_row, d_col, {
            end_col       = d_end_col,
            virt_text     = { { string.rep(" ", width) } },
            virt_text_pos = "overlay",
            priority      = 110,
          })
        end
      end
    end
  end
end

return M
