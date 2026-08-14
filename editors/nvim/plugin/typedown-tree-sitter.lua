if vim.g.typedown_tree_sitter_loaded then return end
vim.g.typedown_tree_sitter_loaded = true

require("typedown.theme").setup()

local release = require("typedown.release")

local PARSER_NAMES = { 'typedown', 'typedown_md', 'typedown_md_inline', 'typedown_yaml' }

-- Resolve tree-sitter parser .so paths, downloading them if necessary.
-- Returns a table mapping parser name to .so path, or nil on failure.
local function resolve_tree_sitter_binaries()
  if vim.g.typedown_dev then
    local root = release.repo_root()
    local result = {}
    for _, name in ipairs(PARSER_NAMES) do
      result[name] = root .. "/packages/tree-sitter/dist/tree-sitter-so/" .. name .. ".so"
    end
    return result
  end

  local tag, version = release.release_tag()

  -- Artifact naming: typedown-treesitter-{grammar}-{version}-{os}-{arch}.so
  local os_arch, err = release.os_arch()
  if not os_arch then
    vim.notify("[typedown] Unsupported platform: " .. err, vim.log.levels.ERROR)
    return nil
  end

  local cache_dir = release.cache_dir(version) .. "/parsers"
  local base_url = release.release_base_url(tag)

  -- Return cached binaries if already downloaded
  if vim.uv.fs_stat(cache_dir) then
    local result = {}
    for _, name in ipairs(PARSER_NAMES) do
      result[name] = cache_dir .. "/typedown-treesitter-" .. name .. "-" .. version .. "-" .. os_arch .. ".so"
    end
    return result
  end

  vim.fn.mkdir(cache_dir, "p")
  vim.notify("[typedown] Downloading Typedown tree-sitter " .. version .. "...", vim.log.levels.INFO)

  local result = {}
  for _, name in ipairs(PARSER_NAMES) do
    local filename = "typedown-treesitter-" .. name .. "-" .. version .. "-" .. os_arch .. ".so"
    local dest = cache_dir .. "/" .. filename
    local url = base_url .. "/" .. filename
    local ok, download_err = release.download(url, dest)
    if not ok then
      vim.fn.delete(cache_dir, 'rf')
      vim.notify("[typedown] Download failed: " .. download_err, vim.log.levels.ERROR)
      return nil
    end
    result[name] = dest
  end

  return result
end

local binaries = resolve_tree_sitter_binaries()

if binaries then
  for parser_name, binary_path in pairs(binaries) do
    vim.treesitter.language.add(parser_name, { path = binary_path })
  end

  vim.treesitter.language.register('typedown', 'typedown')
end
