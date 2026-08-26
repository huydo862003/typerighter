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

local function start_lsp()
  if not binary then return end
  local root = vim.fs.root(0, { "typedown.yaml", "typedown.yml" })
      or vim.fn.fnamemodify(vim.api.nvim_buf_get_name(0), ":h")
  vim.lsp.start({
    name = "typedown-lsp",
    cmd = { binary },
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
