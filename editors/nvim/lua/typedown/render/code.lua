-- Code block renderer: background + left border + language badge + padding
local state = require("typedown.render.state")

local M = {}
M.name = "code"

local LEFT_PAD = 1   -- cells of padding on the left inside the block
local MIN_WIDTH = 40  -- minimum block width in cells

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

  local q = get_query()
  for _, node in q:iter_captures(md_tree:root(), buf) do
    M.render_block(buf, ns, node)
  end
end

function M.render_block(buf, ns, node)
  local start_row, _, end_row = node:range()
  local hidden = state.is_hidden

  -- Calculate max content width for the block
  local max_width = 0
  for row = start_row, end_row - 1 do
    local line = vim.api.nvim_buf_get_lines(buf, row, row + 1, false)[1] or ""
    max_width = math.max(max_width, vim.fn.strdisplaywidth(line))
  end
  local block_width = math.max(max_width + LEFT_PAD * 2, MIN_WIDTH)

  for row = start_row, end_row - 1 do
    -- Background with hl_eol (always visible)
    vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
      end_row  = row + 1,
      hl_group = "TypedownRenderCodeBlock",
      hl_eol   = true,
      priority = 80,
    })

    -- Right-side padding to extend background to block_width
    local line = vim.api.nvim_buf_get_lines(buf, row, row + 1, false)[1] or ""
    local line_width = vim.fn.strdisplaywidth(line)
    local right_pad = math.max(0, block_width - line_width)
    if right_pad > 0 then
      vim.api.nvim_buf_set_extmark(buf, ns, row, #line, {
        virt_text     = { { string.rep(" ", right_pad), "TypedownRenderCodeBlock" } },
        virt_text_pos = "inline",
        priority      = 80,
      })
    end

    -- Left border (hidden on cursor line)
    if not hidden(buf, row) then
      vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
        virt_text     = { { "▎", "TypedownRenderCodeBorder" } },
        virt_text_pos = "inline",
        priority      = 90,
      })
    end
  end

  -- Language badge on opening fence line (hidden on cursor line)
  if not hidden(buf, start_row) then
    local lang_nodes = node:field("language")
    if lang_nodes and lang_nodes[1] then
      local lang = vim.treesitter.get_node_text(lang_nodes[1], buf):lower()
      local icon = LANG_ICONS[lang]
      local badge = icon and (icon .. " " .. lang) or lang
      if badge ~= "" then
        vim.api.nvim_buf_set_extmark(buf, ns, start_row, 0, {
          virt_text     = { { " " .. badge .. " ", "TypedownRenderCodeLanguage" } },
          virt_text_pos = "eol",
          priority      = 100,
        })
      end
    end
  end
end

return M
