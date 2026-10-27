-- Code block renderer: left border + language icon, inline code background
local state = require("typedown.render.state")

local M = {}

-- Common language icons (Nerd Font)
local LANG_ICONS = {
  js = "", javascript = "", ts = "", typescript = "",
  py = "", python = "", rs = "", rust = "",
  go = "", lua = "", rb = "", ruby = "",
  java = "", c = "", cpp = "", cs = "󰌛",
  html = "", css = "", json = "", yaml = "",
  sh = "", bash = "", zsh = "", fish = "",
  sql = "", graphql = "󰡷", dockerfile = "󰡨",
  markdown = "", md = "", toml = "",
}

function M.render(buf, md_tree)
  local ns = state.get_namespace("code")
  state.clear(buf, "code")
  local cursor_line = state.get_cursor_line(buf)

  for node in md_tree:root():iter_children() do
    M.render_node(buf, ns, node, cursor_line)
  end
end

function M.render_node(buf, ns, node, cursor_line)
  if node:type() == "section" then
    for child in node:iter_children() do
      M.render_node(buf, ns, child, cursor_line)
    end
    return
  end

  if node:type() ~= "fenced_code_block" then return end

  local start_row, _, end_row = node:range()

  -- Background for the whole block
  for row = start_row, end_row - 1 do
    vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
      end_row = row + 1,
      hl_group = "TypedownRenderCodeBlock",
      hl_eol = true,
      priority = 80,
    })
  end

  -- Left border
  for row = start_row, end_row - 1 do
    if row ~= cursor_line then
      vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
        virt_text = { { "▎", "TypedownRenderCodeBorder" } },
        virt_text_pos = "inline",
        priority = 90,
      })
    end
  end

  -- Language icon on opening fence line
  local lang_node = node:field("language")
  if lang_node and lang_node[1] and start_row ~= cursor_line then
    local lang = vim.treesitter.get_node_text(lang_node[1], buf):lower()
    -- Strip line range indicator like {1,3}
    lang = lang:gsub("{.*", "")
    local icon = LANG_ICONS[lang]
    if icon then
      vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
        virt_text = { { " " .. icon .. " ", "TypedownRenderCodeLanguage" } },
        virt_text_pos = "eol",
        priority = 100,
      })
    end
  end
end

return M
