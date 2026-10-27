-- Code block renderer: left border + language icon
local state = require("typedown.render.state")

local M = {}
M.name = "code"

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

local query

local function get_query()
  if not query then
    query = vim.treesitter.query.parse("typedown_md", [[
      (fenced_code_block) @code_block
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
    M.render_block(buf, ns, node, cursor_line)
  end
end

function M.render_block(buf, ns, node, cursor_line)
  local start_row, _, end_row = node:range()

  -- Background for the whole block
  for row = start_row, end_row - 1 do
    vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
      end_row  = row + 1,
      hl_group = "TypedownRenderCodeBlock",
      hl_eol   = true,
      priority = 80,
    })
  end

  -- Left border
  for row = start_row, end_row - 1 do
    if row ~= cursor_line then
      vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
        virt_text     = { { "▎", "TypedownRenderCodeBorder" } },
        virt_text_pos = "inline",
        priority      = 90,
      })
    end
  end

  -- Language icon on opening fence line
  local lang_nodes = node:field("language")
  if lang_nodes and lang_nodes[1] and start_row ~= cursor_line then
    local lang = vim.treesitter.get_node_text(lang_nodes[1], buf):lower()
    local icon = LANG_ICONS[lang]
    if icon then
      vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
        virt_text     = { { " " .. icon .. " ", "TypedownRenderCodeLanguage" } },
        virt_text_pos = "eol",
        priority      = 100,
      })
    end
  end
end

return M
