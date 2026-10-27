if vim.g.typedown_render_loaded then return end
vim.g.typedown_render_loaded = true

local render = require("typedown.render")

render.setup()

vim.api.nvim_create_autocmd("FileType", {
  pattern = "typedown",
  callback = function(event)
    render.attach(event.buf)
  end,
})
