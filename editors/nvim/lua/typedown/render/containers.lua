-- Container block renderer: icon + colored left border for ::: blocks
local state = require("typedown.render.state")

local M = {}

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

function M.render(buf, md_tree)
  local ns = state.get_namespace("containers")
  state.clear(buf, "containers")
  local cursor_line = state.get_cursor_line(buf)

  for node in md_tree:root():iter_children() do
    M.render_node(buf, ns, node, cursor_line)
  end
end

function M.render_node(buf, ns, node, cursor_line)
  -- Recurse into sections
  if node:type() == "section" then
    for child in node:iter_children() do
      M.render_node(buf, ns, child, cursor_line)
    end
    return
  end

  if node:type() ~= "container_block" then return end

  local start_row, _, end_row = node:range()

  -- Find container type
  local type_node = node:field("type")[1]
  local container_type = type_node and vim.treesitter.get_node_text(type_node, buf) or ""
  local style = CONTAINER_STYLES[container_type] or DEFAULT_STYLE

  -- Left border on each line
  for row = start_row, end_row - 1 do
    if row ~= cursor_line then
      vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
        virt_text = { { "┃ ", style.hl } },
        virt_text_pos = "inline",
        priority = 90,
      })
    end
  end

  -- Icon on the opening ::: line
  if start_row ~= cursor_line then
    -- Find and conceal the ::: delimiter
    for child in node:iter_children() do
      if child:type() == "container_block_delimiter" then
        local d_row, d_col, _, d_end_col = child:range()
        if d_row == start_row then
          vim.api.nvim_buf_set_extmark(buf, ns, d_row, d_col, {
            end_col = d_end_col,
            conceal = "",
          })
          -- Insert icon
          vim.api.nvim_buf_set_extmark(buf, ns, d_row, d_col, {
            virt_text = { { style.icon .. " ", style.hl } },
            virt_text_pos = "inline",
            priority = 110,
          })
        end
        -- Conceal closing :::
        if d_row == end_row - 1 then
          vim.api.nvim_buf_set_extmark(buf, ns, d_row, d_col, {
            end_col = d_end_col,
            conceal = "",
          })
        end
        break
      end
    end
  end
end

return M
