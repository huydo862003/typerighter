-- Table renderer: box-drawing borders + cell alignment via overlay
local state = require("typedown.render.state")

local M = {}
M.name = "tables"

local BORDER = {
  horizontal = "─",
  vertical   = "│",
  top_left   = "┌", top_mid    = "┬", top_right    = "┐",
  mid_left   = "├", mid_mid    = "┼", mid_right    = "┤",
  bot_left   = "└", bot_mid    = "┴", bot_right    = "┘",
}

local query

local function get_query()
  if not query then
    query = vim.treesitter.query.parse("typedown_md", [[
      (pipe_table) @table
    ]])
  end
  return query
end

-- Parse the delimiter row to get column widths and alignments.
-- Returns { { width = N, align = "left"|"right"|"center"|"default" }, ... }
local function parse_columns(line)
  local cols = {}
  -- Split by | and process each segment
  for segment in line:gmatch("|([^|]+)") do
    local trimmed = segment:match("^%s*(.-)%s*$")
    if trimmed:match("^%-+$") or trimmed:match("^:%-+$") or trimmed:match("^%-+:$") or trimmed:match("^:%-+:$") then
      local width = #segment -- raw segment width including spaces
      local align = "default"
      if trimmed:match("^:.*:$") then
        align = "center"
      elseif trimmed:match("^:") then
        align = "left"
      elseif trimmed:match(":$") then
        align = "right"
      end
      cols[#cols + 1] = { width = width, align = align }
    end
  end
  return cols
end

-- Build an aligned cell overlay: pad content within col_width based on alignment
local function align_cell(content, col_width, align, hl)
  local text = content:match("^%s*(.-)%s*$") or content
  local text_width = vim.fn.strdisplaywidth(text)
  local fill = math.max(0, col_width - text_width)

  local left, right
  if align == "center" then
    left = math.floor(fill / 2)
    right = fill - left
  elseif align == "right" then
    left = fill
    right = 0
  else
    left = 0
    right = fill
  end

  local result = string.rep(" ", left) .. text .. string.rep(" ", right)
  if hl then
    return { result, hl }
  end
  return { result }
end

function M.render(buf, md_tree)
  local ns = state.get_namespace(M.name)
  state.clear(buf, M.name)

  local q = get_query()
  for _, node in q:iter_captures(md_tree:root(), buf) do
    M.render_table(buf, ns, node)
  end
end

function M.render_table(buf, ns, node)
  local start_row, _, end_row = node:range()
  local hidden = state.is_hidden

  -- Find delimiter row and parse columns
  local cols
  local delim_row
  for row = start_row, end_row - 1 do
    local line = vim.api.nvim_buf_get_lines(buf, row, row + 1, false)[1] or ""
    if line:match("^[|%-%s:]+$") then
      cols = parse_columns(line)
      delim_row = row
      break
    end
  end
  if not cols or #cols == 0 then return end

  for row = start_row, end_row - 1 do
    local line = vim.api.nvim_buf_get_lines(buf, row, row + 1, false)[1] or ""
    local is_delimiter = row == delim_row
    local is_header = row == start_row

    if hidden(buf, row) then
      -- Only apply background on hidden (cursor) rows
      vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
        end_row  = row + 1,
        hl_group = is_header and "TypedownRenderTableHeader" or nil,
        hl_eol   = true,
        priority = 80,
      })
    elseif is_delimiter then
      -- Build delimiter overlay with junction characters
      local overlay = {}
      local segments = {}
      for segment in line:gmatch("|([^|]*)") do
        segments[#segments + 1] = segment
      end
      -- leading |
      overlay[#overlay + 1] = { BORDER.mid_left, "TypedownRenderTableBorder" }
      for i, seg in ipairs(segments) do
        if i > #cols + 1 then break end
        if i <= #cols then
          -- Fill with horizontal lines
          local indicator = cols[i].align
          local w = #seg
          if indicator == "left" then
            overlay[#overlay + 1] = { ":", "TypedownRenderTableBorder" }
            overlay[#overlay + 1] = { string.rep(BORDER.horizontal, w - 1), "TypedownRenderTableBorder" }
          elseif indicator == "right" then
            overlay[#overlay + 1] = { string.rep(BORDER.horizontal, w - 1), "TypedownRenderTableBorder" }
            overlay[#overlay + 1] = { ":", "TypedownRenderTableBorder" }
          elseif indicator == "center" then
            overlay[#overlay + 1] = { ":", "TypedownRenderTableBorder" }
            overlay[#overlay + 1] = { string.rep(BORDER.horizontal, math.max(0, w - 2)), "TypedownRenderTableBorder" }
            overlay[#overlay + 1] = { ":", "TypedownRenderTableBorder" }
          else
            overlay[#overlay + 1] = { string.rep(BORDER.horizontal, w), "TypedownRenderTableBorder" }
          end
          -- separator between columns
          if i < #cols then
            overlay[#overlay + 1] = { BORDER.mid_mid, "TypedownRenderTableBorder" }
          end
        end
      end
      -- trailing |
      overlay[#overlay + 1] = { BORDER.mid_right, "TypedownRenderTableBorder" }

      vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
        end_col       = #line,
        virt_text     = overlay,
        virt_text_pos = "overlay",
        priority      = 110,
      })
    else
      -- Data/header row: overlay pipes and align cells
      local segments = {}
      for segment in line:gmatch("|([^|]*)") do
        segments[#segments + 1] = segment
      end
      local hl = is_header and "TypedownRenderTableHeader" or nil

      local overlay = {}
      overlay[#overlay + 1] = { BORDER.vertical, "TypedownRenderTableBorder" }
      for i, seg in ipairs(segments) do
        if i > #cols + 1 then break end
        if i <= #cols then
          overlay[#overlay + 1] = align_cell(seg, cols[i].width, cols[i].align, hl)
          if i < #cols then
            overlay[#overlay + 1] = { BORDER.vertical, "TypedownRenderTableBorder" }
          end
        end
      end
      overlay[#overlay + 1] = { BORDER.vertical, "TypedownRenderTableBorder" }

      vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
        end_col       = #line,
        virt_text     = overlay,
        virt_text_pos = "overlay",
        priority      = 110,
      })

      if is_header then
        vim.api.nvim_buf_set_extmark(buf, ns, row, 0, {
          end_row  = row + 1,
          hl_group = "TypedownRenderTableHeader",
          hl_eol   = true,
          priority = 80,
        })
      end
    end
  end
end

return M
