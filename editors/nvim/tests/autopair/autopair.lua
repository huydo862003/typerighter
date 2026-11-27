-- Test auto-closing pairs and ::: fence in ftplugin/td.lua
local buf = vim.api.nvim_create_buf(false, true)
vim.api.nvim_set_current_buf(buf)
vim.bo[buf].filetype = 'td'

local function feed(keys)
  local termcodes = vim.api.nvim_replace_termcodes(keys, true, false, true)
  vim.api.nvim_feedkeys(termcodes, 'tx', false)
end

-- 1. Typing ::: inserts closing ::: on next line, cursor stays at end of opening :::
feed('i:::')
local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
assert(lines[1] == ':::', 'expected line 1 to be ":::", got: ' .. vim.inspect(lines))
assert(lines[2] == ':::', 'expected line 2 to be ":::", got: ' .. vim.inspect(lines))

-- 2. Typing " note<CR>" after ::: does NOT duplicate :::
feed(' note<CR>')
lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
assert(#lines == 3, 'expected 3 lines, got: ' .. vim.inspect(lines))
assert(lines[1] == '::: note', 'expected line 1 "::: note", got: ' .. vim.inspect(lines))
assert(lines[2] == '', 'expected line 2 "", got: ' .. vim.inspect(lines))
assert(lines[3] == ':::', 'expected line 3 ":::", got: ' .. vim.inspect(lines))

-- 3. Auto-pair (), [[]], ${}, "", ''
vim.api.nvim_buf_set_lines(buf, 0, -1, false, { '' })
vim.api.nvim_win_set_cursor(0, { 1, 0 })
feed('i(')
assert(vim.api.nvim_get_current_line() == '()', 'expected (), got: ' .. vim.api.nvim_get_current_line())
feed('<Esc>A')

vim.api.nvim_buf_set_lines(buf, 0, -1, false, { '' })
vim.api.nvim_win_set_cursor(0, { 1, 0 })
feed('i[[')
assert(vim.api.nvim_get_current_line() == '[[]]', 'expected [[]], got: ' .. vim.api.nvim_get_current_line())

vim.api.nvim_buf_set_lines(buf, 0, -1, false, { '' })
vim.api.nvim_win_set_cursor(0, { 1, 0 })
feed('i${')
assert(vim.api.nvim_get_current_line() == '${}', 'expected ${}, got: ' .. vim.api.nvim_get_current_line())

vim.api.nvim_buf_set_lines(buf, 0, -1, false, { '' })
vim.api.nvim_win_set_cursor(0, { 1, 0 })
feed('i"')
assert(vim.api.nvim_get_current_line() == '""', 'expected "", got: ' .. vim.api.nvim_get_current_line())

vim.api.nvim_buf_set_lines(buf, 0, -1, false, { '' })
vim.api.nvim_win_set_cursor(0, { 1, 0 })
feed("i'")
assert(vim.api.nvim_get_current_line() == "''", "expected '', got: " .. vim.api.nvim_get_current_line())

print('All autopair tests passed!')
