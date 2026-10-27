-- Test helper for headless Neovim LSP integration tests
-- Usage: nvim --headless -u test/minimal_init.lua -l test/helper.lua -l test/diagnostics.lua

local M = {}

--- Wait for a condition to become true, with timeout
---@param condition fun(): boolean
---@param timeout_ms? number (default 10000)
function M.wait_for(condition, timeout_ms)
  timeout_ms = timeout_ms or 10000
  vim.wait(timeout_ms, condition, 50)
end

--- Wait for LSP client to attach to the current buffer
---@param timeout_ms? number
---@return vim.lsp.Client|nil
function M.wait_for_lsp(timeout_ms)
  timeout_ms = timeout_ms or 10000
  local client
  vim.wait(timeout_ms, function()
    local clients = vim.lsp.get_clients({ name = "typedown-lsp" })
    if #clients > 0 then
      client = clients[1]
      return true
    end
    return false
  end, 50)
  return client
end

--- Wait for diagnostics to appear on the current buffer
---@param timeout_ms? number
---@return vim.Diagnostic[]
function M.wait_for_diagnostics(timeout_ms)
  timeout_ms = timeout_ms or 10000
  local diags = {}
  vim.wait(timeout_ms, function()
    diags = vim.diagnostic.get(0)
    return #diags > 0
  end, 50)
  return diags
end

--- Wait for diagnostics to be empty on the current buffer
---@param timeout_ms? number
---@return vim.Diagnostic[]
function M.wait_for_no_diagnostics(timeout_ms)
  timeout_ms = timeout_ms or 10000
  -- Give the server time to publish diagnostics first
  vim.wait(2000, function() return false end, 50)
  return vim.diagnostic.get(0)
end

--- Make a synchronous LSP request
---@param method string
---@param params? table
---@param timeout_ms? number
---@return any result, any error
function M.lsp_request(method, params, timeout_ms)
  timeout_ms = timeout_ms or 5000
  local client = M.wait_for_lsp()
  assert(client, "LSP client not attached")

  local result = client:request_sync(method, params, timeout_ms, vim.api.nvim_get_current_buf())
  if result and result.err then
    return nil, result.err
  end
  return result and result.result, nil
end

--- Open a file from the fixtures directory
---@param vault string vault name (e.g. "basic-vault")
---@param file string relative file path
function M.open_fixture(vault, file)
  local script_dir = debug.getinfo(1, "S").source:sub(2):match("(.*/)")
  local path = script_dir .. "fixtures/" .. vault .. "/" .. file
  vim.cmd("edit " .. vim.fn.fnameescape(path))
  -- Wait for filetype detection and LSP attach
  vim.wait(500, function() return false end, 50)
end

--- Assert helper
---@param condition any
---@param message string
function M.assert(condition, message)
  if not condition then
    io.stderr:write("FAIL: " .. message .. "\n")
    vim.cmd("cquit 1")
  end
end

--- Assert equality
---@param actual any
---@param expected any
---@param message string
function M.assert_eq(actual, expected, message)
  if actual ~= expected then
    io.stderr:write(string.format("FAIL: %s\n  expected: %s\n  actual:   %s\n", message, vim.inspect(expected), vim.inspect(actual)))
    vim.cmd("cquit 1")
  end
end

--- Run a named test
---@param name string
---@param fn fun()
function M.test(name, fn)
  local ok, err = pcall(fn)
  if ok then
    io.write("  PASS: " .. name .. "\n")
  else
    io.stderr:write("  FAIL: " .. name .. ": " .. tostring(err) .. "\n")
    -- Don't exit immediately - run remaining tests
    M._has_failures = true
  end
end

M._has_failures = false

--- Call at end of test file to exit with correct code
function M.finish()
  -- Give LSP time to shut down cleanly
  for _, client in ipairs(vim.lsp.get_clients()) do
    client:stop()
  end
  vim.wait(500, function() return false end, 50)

  if M._has_failures then
    vim.cmd("cquit 1")
  else
    vim.cmd("qall!")
  end
end

return M
