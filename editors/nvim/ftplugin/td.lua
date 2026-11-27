vim.bo.tabstop = 2
vim.bo.shiftwidth = 2
vim.bo.expandtab = true

-- List continuation: same settings as the built-in markdown ftplugin
vim.bo.comments = "fb:*,fb:-,fb:+,n:>"
vim.bo.formatlistpat = table.concat({
  [[^\s*\d\+\.\s\+]],           -- numbered lists (e.g. "1. ")
  [[^\s*[-*+]\s\+]],            -- bullet lists ("- ", "* ", "+ ")
  [=[^\[^\ze[^\]]\+\]:\&^.\{4\}]=], -- footnote definitions ("[^id]:")
}, [[\|]])
-- n: recognize numbered lists, r: auto-insert list marker on enter
vim.bo.formatoptions = vim.bo.formatoptions .. "rn"

-- Auto-close code fences and math blocks on Enter.
-- When pressing Enter at the end of a ``` or $$ line, insert:
-- <empty line>     <- cursor lands here
-- closing fence
vim.keymap.set('i', '<CR>', function()
  local line = vim.api.nvim_get_current_line()
  local row, col = unpack(vim.api.nvim_win_get_cursor(0))
  -- Only trigger when cursor is at end of a fence-opening line
  if not line:sub(col + 1):match('^%s*$') then return '<CR>' end
  local next_line = vim.api.nvim_buf_get_lines(0, row, row + 1, false)[1]
  local close = line:match('^%s*```') and '```'
    or line:match('^%s*%$%$$') and '$$'
    or (line:match('^%s*:::') and not (next_line and next_line:match('^%s*:::'))) and ':::'
  if not close then return '<CR>' end
  local indent = line:match('^(%s*)') or ''
  local keys = '<CR><CR>' .. indent .. close .. '<Up>'
  return vim.api.nvim_replace_termcodes(keys, true, false, true)
end, { buffer = true, expr = true })

-- Auto-close ::: when typing the 3rd colon on a line
vim.keymap.set('i', ':', function()
  local line = vim.api.nvim_get_current_line()
  local col = vim.api.nvim_win_get_cursor(0)[2]
  local before = line:sub(1, col)
  local after = line:sub(col + 1)
  local indent = before:match('^(%s*)::$')
  if indent and after:match('^%s*$') then
    return vim.api.nvim_replace_termcodes(':<CR>' .. indent .. ':::<Up><End>', true, false, true)
  end
  return ':'
end, { buffer = true, expr = true })

-- Create a new .td file and insert fref() at cursor
-- Filters code actions to only show create-linked and backward-link actions
vim.api.nvim_buf_create_user_command(0, 'TypedownCreateLinkedResource', function()
  vim.lsp.buf.code_action({
    filter = function(action)
      return action.title and (
        action.title:find("Create new.*link") or
        action.title:find("linking back")
      )
    end,
    apply = true,
  })
end, { desc = "Create linked resource" })

-- Auto-pair $ for inline math: insert $|$ or skip over closing $
vim.keymap.set('i', '$', function()
  local line = vim.api.nvim_get_current_line()
  local col = vim.api.nvim_win_get_cursor(0)[2]
  if line:sub(col + 1, col + 1) == '$' then
    return vim.api.nvim_replace_termcodes('<Right>', true, false, true)
  end
  return vim.api.nvim_replace_termcodes('$$<Left>', true, false, true)
end, { buffer = true, expr = true })

-- Auto-pair (), [], {}, "", '' and ${} (converting $|$ to ${|})
local function delegate_or(lhs, fallback)
  local existing = vim.fn.maparg(lhs, 'i', false, true)
  if existing and not vim.tbl_isempty(existing) then
    if existing.expr == 1 and existing.callback then
      return existing.callback()
    end
    if existing.rhs and existing.rhs ~= '' then
      return vim.api.nvim_replace_termcodes(existing.rhs, true, false, true)
    end
  end
  return fallback()
end

local function auto_pair(open, close)
  local existing_open = vim.fn.maparg(open, 'i', false, true)
  if open == '{' or not existing_open or vim.tbl_isempty(existing_open) then
    vim.keymap.set('i', open, function()
      local line = vim.api.nvim_get_current_line()
      local col = vim.api.nvim_win_get_cursor(0)[2]
      local prev_char = col > 0 and line:sub(col, col) or ''
      local next_char = line:sub(col + 1, col + 1)
      if open == '{' and prev_char == '$' and next_char == '$' then
        return vim.api.nvim_replace_termcodes('<Del>{}<Left>', true, false, true)
      end
      return delegate_or(open, function()
        if open == close and next_char == close then
          return vim.api.nvim_replace_termcodes('<Right>', true, false, true)
        end
        return vim.api.nvim_replace_termcodes(open .. close .. '<Left>', true, false, true)
      end)
    end, { buffer = true, expr = true })
  end

  local existing_close = vim.fn.maparg(close, 'i', false, true)
  if open ~= close and (not existing_close or vim.tbl_isempty(existing_close)) then
    vim.keymap.set('i', close, function()
      local line = vim.api.nvim_get_current_line()
      local col = vim.api.nvim_win_get_cursor(0)[2]
      if line:sub(col + 1, col + 1) == close then
        return vim.api.nvim_replace_termcodes('<Right>', true, false, true)
      end
      return close
    end, { buffer = true, expr = true })
  end
end

auto_pair('(', ')')
auto_pair('[', ']')
auto_pair('{', '}')
auto_pair('"', '"')
auto_pair("'", "'")
