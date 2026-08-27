vim.bo.tabstop = 2
vim.bo.shiftwidth = 2
vim.bo.expandtab = true

-- List continuation: same settings as the built-in markdown ftplugin
vim.bo.comments = "fb:*,fb:-,fb:+,n:>"
vim.bo.formatlistpat = [[^\s*\d\+\.\s\+\|^\s*[-*+]\s\+\|^\[^\ze[^\]]\+\]:\&^.\{4\}]]
-- n: recognize numbered lists, r: auto-insert list marker on enter
vim.bo.formatoptions = vim.bo.formatoptions .. "rn"
