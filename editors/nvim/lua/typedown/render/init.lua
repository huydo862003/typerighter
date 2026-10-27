-- Main rendering engine: attach/detach per buffer, orchestrate renderers

local state = require("typedown.render.state")
local highlights = require("typedown.render.highlights")
local anti_conceal = require("typedown.render.anti-conceal")
local headings = require("typedown.render.headings")
local containers = require("typedown.render.containers")
local checkboxes = require("typedown.render.checkboxes")
local bullets = require("typedown.render.bullets")
local code = require("typedown.render.code")
local tables = require("typedown.render.tables")
local toggle_list = require("typedown.render.toggle-list")

local M = {}

local renderers = {
  headings,
  containers,
  checkboxes,
  bullets,
  code,
  tables,
  toggle_list,
}

function M.setup()
  highlights.setup()
  checkboxes.setup_commands()
  toggle_list.setup_commands()
end

function M.render_buf(buf)
  local parser = vim.treesitter.get_parser(buf, "typedown")
  if not parser then return end

  parser:parse()

  -- Find the typedown_md subtree (injected from the body)
  for _, child_tree in pairs(parser:children()) do
    if child_tree:lang() == "typedown_md" then
      for _, tree in ipairs(child_tree:trees()) do
        for _, renderer in ipairs(renderers) do
          local ok, err = pcall(renderer.render, buf, tree)
          if not ok then
            vim.schedule(function()
              vim.notify("[typedown render] " .. tostring(err), vim.log.levels.DEBUG)
            end)
          end
        end
      end
      return
    end
  end
end

function M.attach(buf)
  if state.is_attached(buf) then return end
  state.mark_attached(buf)

  -- Initial cursor position
  local ok, cursor = pcall(vim.api.nvim_win_get_cursor, 0)
  if ok then
    state.set_cursor_line(buf, cursor[1] - 1)
  end

  -- Initial render (deferred to let tree-sitter parse first)
  vim.defer_fn(function()
    if vim.api.nvim_buf_is_valid(buf) then
      M.render_buf(buf)
    end
  end, 100)

  -- Re-render on text changes
  vim.api.nvim_buf_attach(buf, false, {
    on_lines = function()
      vim.defer_fn(function()
        if vim.api.nvim_buf_is_valid(buf) then
          M.render_buf(buf)
        end
      end, 50)
    end,
    on_detach = function()
      state.detach(buf)
    end,
  })

  -- Anti-conceal: re-render on cursor move
  local group = vim.api.nvim_create_augroup("typedown_render_" .. buf, { clear = true })

  vim.api.nvim_create_autocmd("CursorMoved", {
    group = group,
    buffer = buf,
    callback = function()
      if anti_conceal.on_cursor_moved(buf) then
        M.render_buf(buf)
      end
    end,
  })

  vim.api.nvim_create_autocmd("InsertEnter", {
    group = group,
    buffer = buf,
    callback = function()
      if anti_conceal.on_insert_enter(buf) then
        -- Clear all rendering in insert mode
        for _, renderer_name in ipairs({ "headings", "containers", "checkboxes", "bullets", "code", "tables", "toggle_list" }) do
          state.clear(buf, renderer_name)
        end
      end
    end,
  })

  vim.api.nvim_create_autocmd("InsertLeave", {
    group = group,
    buffer = buf,
    callback = function()
      if anti_conceal.on_insert_leave(buf) then
        M.render_buf(buf)
      end
    end,
  })

  vim.api.nvim_create_autocmd("BufDelete", {
    group = group,
    buffer = buf,
    callback = function()
      state.detach(buf)
      pcall(vim.api.nvim_del_augroup_by_id, group)
    end,
  })
end

return M
