-- Per-buffer rendering state management
-- Each renderer gets its own extmark namespace for independent clear/update

local M = {}

-- Namespace registry: renderer_name -> namespace_id
local namespaces = {}

-- Per-buffer state: buf -> { cursor_line, attached }
local buf_state = {}

function M.get_namespace(name)
  if not namespaces[name] then
    namespaces[name] = vim.api.nvim_create_namespace("typedown_render_" .. name)
  end
  return namespaces[name]
end

function M.clear(buf, name)
  local ns = M.get_namespace(name)
  vim.api.nvim_buf_clear_namespace(buf, ns, 0, -1)
end

function M.get_buf_state(buf)
  if not buf_state[buf] then
    buf_state[buf] = { cursor_line = -1, attached = false }
  end
  return buf_state[buf]
end

function M.set_cursor_line(buf, line)
  local state = M.get_buf_state(buf)
  state.cursor_line = line
end

function M.get_cursor_line(buf)
  return M.get_buf_state(buf).cursor_line
end

function M.mark_attached(buf)
  M.get_buf_state(buf).attached = true
end

function M.is_attached(buf)
  return M.get_buf_state(buf).attached
end

function M.detach(buf)
  buf_state[buf] = nil
  for name in pairs(namespaces) do
    pcall(vim.api.nvim_buf_clear_namespace, buf, namespaces[name], 0, -1)
  end
end

-- Get typedown_md tree objects for a buffer
function M.get_md_trees(buf)
  local parser = vim.treesitter.get_parser(buf, "typedown")
  if not parser then return {} end
  parser:parse()

  for _, child_tree in pairs(parser:children()) do
    if child_tree:lang() == "typedown_md" then
      return child_tree:trees()
    end
  end
  return {}
end

return M
