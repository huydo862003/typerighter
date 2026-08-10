-- Paste interception for Typedown files
-- Detects binary/image data on the clipboard and saves it to the assets directory

local sysname = vim.uv.os_uname().sysname

local supported_mime_types = {
  "image/png",
  "image/jpeg",
  "image/svg+xml",
  "image/webp",
  "application/pdf",
}

local mime_to_ext = {
  ["image/png"] = "png",
  ["image/jpeg"] = "jpg",
  ["image/svg+xml"] = "svg",
  ["image/webp"] = "webp",
  ["application/pdf"] = "pdf",
}

-- Check for display server on POSIX systems (wayland/X)
local function detect_display_server()
  local wayland = vim.env.WAYLAND_DISPLAY
  if wayland and wayland ~= "" then
    return "wayland"
  end
  return "x11"
end

-- Detect the media type contained in clipboard
local function detect_clipboard_mime()
  -- Mac
  if sysname == "Darwin" then
    local result = vim.fn.system("osascript -e 'clipboard info' 2>/dev/null")
    if result:find("PDF") then return "application/pdf" end
    if result:find("SVG") then return "image/svg+xml" end
    if result:find("WebP") then return "image/webp" end
    if result:find("JPEG") then return "image/jpeg" end
    if result:find("TIFF") or result:find("PNG") then return "image/png" end
    return nil
  end

  -- POSIX
  local display = detect_display_server()

  -- TIL: Simulate ternary with short-circuiting `and` and `or`
  local targets_cmd = display == "wayland"
    and "wl-paste --list-types 2>/dev/null"
    or "xclip -selection clipboard -t TARGETS -o 2>/dev/null"

  local targets_output = vim.fn.system(targets_cmd)
  if vim.v.shell_error ~= 0 then
    return nil
  end

  for _, mime in ipairs(supported_mime_types) do
    if targets_output:find(mime, 1, true) then
      return mime
    end
  end

  return nil
end

-- Generate the clipboard read command
local function clipboard_read_cmd(mime_type)
  if sysname == "Darwin" then
    if mime_type == "application/pdf" then
      return nil
    end
    return "pngpaste - 2>/dev/null"
  end

  local display = detect_display_server()
  if display == "wayland" then
    return string.format("wl-paste --type '%s' 2>/dev/null", mime_type)
  end
  return string.format("xclip -selection clipboard -t '%s' -o 2>/dev/null", mime_type)
end

-- Read binary from clipboard via io.popen, write to disk via io.open.
-- Returns true on success, false if the clipboard tool or file write fails.
local function save_clipboard_binary(mime_type, dest_path)
  local read_cmd = clipboard_read_cmd(mime_type)
  if not read_cmd then
    return false
  end

  local handle = io.popen(read_cmd, "r")
  if not handle then
    return false
  end

  local data = handle:read("*a")
  handle:close()

  if not data or #data == 0 then
    return false
  end

  local file = io.open(dest_path, "wb")
  if not file then
    return false
  end

  file:write(data)
  file:close()
  return true
end

-- Main paste handler for binary clipboard content.
local function handle_binary_paste(mime_type)
  local bufnr = vim.api.nvim_get_current_buf()
  local params = {
    textDocument = {
      uri = vim.uri_from_bufnr(bufnr),
    },
  }

  vim.lsp.buf_request(
    bufnr,
    "typedown/getAssetsDir",
    params,
    function(err, result)
      -- After requesting assets directory from typedown-lsp...
      if err then
        vim.notify("[typedown] Failed to get assets dir: " .. tostring(err), vim.log.levels.ERROR)
        return
      end

      if not result then
        vim.notify("[typedown] No response from typedown-lsp for assets dir", vim.log.levels.ERROR)
        return
      end

      -- Build the assets path from LSP response (relative subdir name)  & current file location
      local file_path = vim.api.nvim_buf_get_name(bufnr)
      local file_dir = vim.fn.fnamemodify(file_path, ":h") -- parent directory
      local assets_subdir = result.path or "assets"
      local assets_dir = file_dir .. "/" .. assets_subdir

      vim.fn.mkdir(assets_dir, "p") -- "p" = create parents

      -- Save clipboard binary to a <stem>-<timestamp>.<ext> file
      local extension = mime_to_ext[mime_type] or "bin"
      local stem = vim.fn.fnamemodify(file_path, ":t:r") -- filename without extension
      if stem == "" then stem = "untitled" end
      local filename = stem .. "-" .. os.time() .. "." .. extension
      local dest_path = assets_dir .. "/" .. filename

      local success = save_clipboard_binary(mime_type, dest_path)
      if not success then
        vim.notify("[typedown] Failed to save clipboard content to " .. dest_path, vim.log.levels.ERROR)
        return
      end

      -- Insert fref() at cursor position
      local relative_path = assets_subdir .. "/" .. filename
      local fref_text = string.format('${fref("%s")}', relative_path)

      -- Defers to the main loop since we're inside an async LSP callback
      vim.schedule(function()
        local cursor = vim.api.nvim_win_get_cursor(0)
        local row = cursor[1] - 1 -- nvim_win_get_cursor is 1-indexed, but buf_set_lines is 0-indexed @@
        local col = cursor[2]
        local current_line = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)[1] or ""
        local before = current_line:sub(1, col)
        local after = current_line:sub(col + 1)
        vim.api.nvim_buf_set_lines(bufnr, row, row + 1, false, { before .. fref_text .. after })
        vim.api.nvim_win_set_cursor(0, { row + 1, col + #fref_text })
      end)

      vim.notify("[typedown] Saved " .. filename .. " to assets", vim.log.levels.INFO)
  end)
end

-- Entry point for this plugin
local function paste_intercept(fallback_key)
  local mime_type = detect_clipboard_mime()
  if mime_type then
    handle_binary_paste(mime_type)
  else
    -- Feed the original key to Neovim's input queue with "n" to avoid recursion
    local keys = vim.api.nvim_replace_termcodes(fallback_key, true, false, true)
    vim.api.nvim_feedkeys(keys, "n", false)
  end
end

vim.api.nvim_create_autocmd("FileType", {
  pattern = "typedown",
  callback = function(event)
    local opts = { buffer = event.buf, silent = true }

    vim.keymap.set({ "n", "i" }, "<C-S-v>", function()
      paste_intercept("<C-r>+")
    end, vim.tbl_extend("force", opts, { desc = "Typedown paste asset" }))
  end,
})
