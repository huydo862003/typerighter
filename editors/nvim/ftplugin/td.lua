vim.bo.tabstop = 2
vim.bo.shiftwidth = 2
vim.bo.expandtab = true

-- List continuation: same settings as the built-in markdown ftplugin
vim.bo.comments = "fb:*,fb:-,fb:+,n:>"
vim.bo.formatlistpat = [[^\s*\d\+\.\s\+\|^\s*[-*+]\s\+\|^\[^\ze[^\]]\+\]:\&^.\{4\}]]
-- n: recognize numbered lists, r: auto-insert list marker on enter
vim.bo.formatoptions = vim.bo.formatoptions .. "rn"

-- Auto-close code fences and math blocks on Enter.
-- When pressing Enter at the end of a ``` or $$ line, insert:
-- <empty line>     <- cursor lands here
-- closing fence
vim.keymap.set('i', '<CR>', function()
  local line = vim.api.nvim_get_current_line()
  local col = vim.api.nvim_win_get_cursor(0)[2]
  -- Only trigger when cursor is at end of a fence-opening line
  if not line:sub(col + 1):match('^%s*$') then return '<CR>' end
  local close = line:match('^%s*```') and '```' or line:match('^%s*%$%$$') and '$$'
  if not close then return '<CR>' end
  local indent = line:match('^(%s*)') or ''
  local keys = '<CR><CR>' .. indent .. close .. '<Up>'
  return vim.api.nvim_replace_termcodes(keys, true, false, true)
end, { buffer = true, expr = true })

-- Auto-pair $ for inline math: insert $|$ or skip over closing $
vim.keymap.set('i', '$', function()
  local line = vim.api.nvim_get_current_line()
  local col = vim.api.nvim_win_get_cursor(0)[2]
  if line:sub(col + 1, col + 1) == '$' then
    return vim.api.nvim_replace_termcodes('<Right>', true, false, true)
  end
  return vim.api.nvim_replace_termcodes('$$<Left>', true, false, true)
end, { buffer = true, expr = true })
