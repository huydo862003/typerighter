if vim.g.typedown_lsp_loaded then return end
vim.g.typedown_lsp_loaded = true

local release = require("typedown.release")

-- Resolve the LSP binary, downloading it if necessary.
-- Returns the binary path, or nil if it could not be resolved.
local function resolve_lsp_binary()
  if vim.g.typedown_dev then
    return release.repo_root() .. "/target/debug/typedown-lsp"
  end

  -- Prefer Nix-provided binary if available
  local nix_binary = vim.fn.exepath("typedown-lsp")
  if nix_binary ~= "" then
    return nix_binary
  end

  local tag, version = release.release_tag()

  -- Artifact naming: typedown-lsp-{version}-{os}-{arch}[.exe]
  local os_arch, err = release.os_arch()
  if not os_arch then
    vim.notify("[typedown] Unsupported platform: " .. err, vim.log.levels.ERROR)
    return nil
  end
  local ext = os_arch:find("^windows") and ".exe" or ""
  local artifact = "typedown-lsp-" .. version .. "-" .. os_arch .. ext

  local cache_dir = release.cache_dir(version)
  local binary = cache_dir .. "/" .. artifact

  if vim.uv.fs_stat(binary) then
    return binary
  end

  vim.fn.mkdir(cache_dir, "p")
  vim.notify("[typedown] Downloading typedown-lsp " .. version .. "...", vim.log.levels.INFO)

  local url = release.release_base_url(tag) .. "/" .. artifact
  local ok, download_err = release.download(url, binary)
  if not ok then
    vim.notify("[typedown] Download failed: " .. download_err, vim.log.levels.ERROR)
    return nil
  end

  vim.uv.fs_chmod(binary, 493) -- 0755
  return binary
end

local binary = resolve_lsp_binary()

-- Generic prompt resolver for typerighter commands
local function resolve_prompts_and_execute(cmd, ctx)
  local args = cmd.arguments and cmd.arguments[1]
  if not args then return end

  local prompts = args.prompts or {}
  if #prompts == 0 then
    -- No prompts remaining, send directly
    local client = vim.lsp.get_client_by_id(ctx.client_id)
    if client then
      client.request("workspace/executeCommand", {
        command = cmd.command,
        arguments = cmd.arguments,
      })
    end
    return
  end

  -- Process the first prompt, then recurse for the rest
  local prompt = table.remove(prompts, 1)

  if prompt.kind == "input" then
    vim.ui.input({ prompt = prompt.prompt .. " ", default = prompt.default or "" }, function(value)
      if not value or value == "" then return end
      args[prompt.field] = value
      resolve_prompts_and_execute(cmd, ctx)
    end)
  elseif prompt.kind == "select" then
    vim.ui.select(prompt.choices, { prompt = prompt.prompt }, function(choice)
      if not choice then return end
      args[prompt.field] = choice
      resolve_prompts_and_execute(cmd, ctx)
    end)
  end
end

-- Track spawned server process and its port
local server_job = nil
local server_port = nil

local function start_lsp()
  if not binary then return end
  local root = vim.fs.root(0, { "typedown.yaml", "typedown.yml" })
      or vim.fn.fnamemodify(vim.api.nvim_buf_get_name(0), ":h")

  -- Spawn the LSP binary if not already running, read port from stdout
  if not server_job then
    local port_received = false
    server_job = vim.fn.jobstart({ binary }, {
      stdout_buffered = false,
      on_stdout = function(_, data)
        if not port_received and data and data[1] and data[1] ~= "" then
          server_port = tonumber(data[1])
          port_received = true
        end
      end,
      on_exit = function()
        server_job = nil
        server_port = nil
      end,
    })
    -- Wait for the port to be printed
    vim.wait(2000, function() return server_port ~= nil end, 10)
    if not server_port then
      vim.notify("[typedown] Failed to get LSP port", vim.log.levels.ERROR)
      return
    end
  end

  vim.lsp.start({
    name = "typedown-lsp",
    cmd = vim.lsp.rpc.connect("127.0.0.1", server_port),
    root_dir = root,
    -- Neovim defaults fileOperations to false
    -- Enabling these lets file managers send workspace/willRenameFiles before renaming .td files,
    -- But there is no truly way to do this :(
    -- See: https://github.com/neovim/neovim/blob/master/runtime/lua/vim/lsp/protocol.lua
    capabilities = vim.tbl_deep_extend("force", vim.lsp.protocol.make_client_capabilities(), {
      workspace = {
        fileOperations = {
          willRename = true,
          didRename = true,
        },
      },
    }),
    handlers = {
      -- After applying a workspace edit
      -- save all modified buffers so the LSP sees the changes via didChange/didOpen
      ["textDocument/rename"] = function(err, result, ctx, config)
        vim.lsp.handlers["textDocument/rename"](err, result, ctx, config)
        if result then
          for _, buf in ipairs(vim.api.nvim_list_bufs()) do
            if vim.api.nvim_buf_is_loaded(buf) and vim.bo[buf].modified then
              vim.api.nvim_buf_call(buf, function() vim.cmd("silent! write") end)
            end
          end
        end
      end,
    },
    -- Route all _typerighter.* commands through the generic prompt resolver
    commands = setmetatable({}, {
      __index = function(_, key)
        if key:find("^_typerighter%.") then
          return resolve_prompts_and_execute
        end
      end,
    }),
  })
end

vim.api.nvim_create_autocmd("FileType", {
  pattern = "typedown",
  callback = function(event)
    start_lsp()

    vim.api.nvim_buf_create_user_command(event.buf, "TypedownFormat", function()
      vim.lsp.buf.format({ name = "typedown-lsp" })
    end, { desc = "Format current typedown file" })

    vim.api.nvim_buf_create_user_command(event.buf, "TypedownLint", function()
      vim.diagnostic.setloclist({ open = true })
    end, { desc = "Show typedown diagnostics in location list" })
  end,
})

vim.api.nvim_create_autocmd("BufEnter", {
  pattern = { "typedown.yaml", "typedown.yml" },
  callback = start_lsp,
})
