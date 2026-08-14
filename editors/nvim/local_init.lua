-- Load the plugin against the local debug build (target/debug/typedown-lsp).
-- Usage: nvim -u editors/nvim/local_init.lua path/to/file.td
vim.g.typedown_dev = true

-- Load local plugins first so they set load guards before the system plugin
local plugin_dir = vim.fn.getcwd() .. "/editors/nvim"
vim.opt.runtimepath:append(plugin_dir)

vim.cmd("source " .. plugin_dir .. "/ftdetect/typedown.lua")
vim.cmd("source " .. plugin_dir .. "/plugin/typedown-lsp.lua")
vim.cmd("source " .. plugin_dir .. "/plugin/typedown-tree-sitter.lua")
vim.cmd("source " .. plugin_dir .. "/plugin/typedown-paste.lua")

vim.cmd("source " .. vim.fn.stdpath("config") .. "/init.lua")
