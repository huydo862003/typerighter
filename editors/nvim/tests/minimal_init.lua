-- Minimal init.lua for headless LSP testing
-- Loads only the typedown plugin, no user config

-- Use the local dev binary
vim.g.typedown_dev = true

-- Add the plugin to runtimepath
local plugin_dir = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":h:h")
vim.opt.runtimepath:prepend(plugin_dir)

-- Register .td filetype
vim.filetype.add({
  extension = {
    td = "typedown",
  },
})
