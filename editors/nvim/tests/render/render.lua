local h = require("tests.helper")

io.write("Render:\n")

h.test("checkbox toggle: unchecked to checked", function()
  vim.cmd("enew")
  vim.bo.filetype = "typedown"
  local buf = vim.api.nvim_get_current_buf()
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, {
    "---",
    "---",
    "- [ ] buy groceries",
    "- [x] done task",
  })

  vim.api.nvim_win_set_cursor(0, { 3, 0 })
  require("typedown.render.checkboxes").toggle()
  local line = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1]
  h.assert_eq(line, "- [x] buy groceries", "should toggle [ ] to [x] and preserve text")
end)

h.test("checkbox toggle: checked to cancelled", function()
  vim.cmd("enew")
  vim.bo.filetype = "typedown"
  local buf = vim.api.nvim_get_current_buf()
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, {
    "---",
    "---",
    "- [x] done task",
  })

  vim.api.nvim_win_set_cursor(0, { 3, 0 })
  require("typedown.render.checkboxes").toggle()
  local line = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1]
  h.assert_eq(line, "- [~] done task", "should toggle [x] to [~] (next in cycle)")
end)

h.test("checkbox toggle: converts plain list item to checkbox", function()
  vim.cmd("enew")
  vim.bo.filetype = "typedown"
  local buf = vim.api.nvim_get_current_buf()
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, {
    "---",
    "---",
    "- plain item",
  })

  vim.api.nvim_win_set_cursor(0, { 3, 0 })
  require("typedown.render.checkboxes").toggle()
  local line = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1]
  h.assert_eq(line, "- [ ] plain item", "should convert plain list item to checkbox")
end)

h.test("checkbox toggle: full cycle returns to unchecked", function()
  vim.cmd("enew")
  vim.bo.filetype = "typedown"
  local buf = vim.api.nvim_get_current_buf()
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, {
    "---",
    "---",
    "- [ ] test item with **bold**",
  })
  local original = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1]

  vim.api.nvim_win_set_cursor(0, { 3, 0 })
  local checkboxes = require("typedown.render.checkboxes")
  -- 5 toggles: [ ] -> [x] -> [~] -> [>] -> [!] -> [ ]
  for _ = 1, 5 do checkboxes.toggle() end
  local after = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1]
  h.assert_eq(after, original, "full cycle (5 toggles) should restore original line")
end)

h.test("highlight groups have correct attributes", function()
  require("typedown.render.highlights").setup()

  local h1 = vim.api.nvim_get_hl(0, { name = "TypedownRenderH1" })
  h.assert(h1.bold, "H1 should be bold")
  h.assert(h1.fg, "H1 should have foreground color")

  local h1bg = vim.api.nvim_get_hl(0, { name = "TypedownRenderH1Bg" })
  h.assert(h1bg.bg, "H1Bg should have background color")
  h.assert(not h1bg.fg, "H1Bg should not have foreground (inherits text color)")

  local note = vim.api.nvim_get_hl(0, { name = "TypedownRenderContainerNote" })
  h.assert(note.fg, "ContainerNote should have foreground color")

  local checked = vim.api.nvim_get_hl(0, { name = "TypedownRenderCheckboxChecked" })
  h.assert(checked.fg, "CheckboxChecked should have foreground color")

  local unchecked = vim.api.nvim_get_hl(0, { name = "TypedownRenderCheckboxUnchecked" })
  h.assert(unchecked.fg ~= checked.fg, "checked and unchecked should have different colors")
end)

h.test("all highlight groups are defined", function()
  require("typedown.render.highlights").setup()

  local groups = {
    "TypedownRenderH1", "TypedownRenderH1Bg",
    "TypedownRenderH2", "TypedownRenderH2Bg",
    "TypedownRenderH3", "TypedownRenderH3Bg",
    "TypedownRenderH4", "TypedownRenderH4Bg",
    "TypedownRenderH5", "TypedownRenderH5Bg",
    "TypedownRenderH6", "TypedownRenderH6Bg",
    "TypedownRenderContainerNote",
    "TypedownRenderContainerWarning",
    "TypedownRenderContainerTip",
    "TypedownRenderContainerImportant",
    "TypedownRenderContainerCaution",
    "TypedownRenderContainerDefault",
    "TypedownRenderCodeBlock",
    "TypedownRenderCodeLanguage",
    "TypedownRenderCodeBorder",
    "TypedownRenderTableBorder",
    "TypedownRenderTableHeader",
    "TypedownRenderCheckboxUnchecked",
    "TypedownRenderCheckboxChecked",
    "TypedownRenderCheckboxCancelled",
    "TypedownRenderCheckboxForwarded",
    "TypedownRenderCheckboxImportant",
    "TypedownRenderFrefIcon",
    "TypedownRenderFrefText",
    "TypedownRenderFrontmatterRule",
    "TypedownRenderFrontmatterContent",
    "TypedownRenderBullet",
    "TypedownRenderToggleOpen",
    "TypedownRenderToggleClosed",
  }

  for _, group in ipairs(groups) do
    local hl = vim.api.nvim_get_hl(0, { name = group })
    h.assert(next(hl) ~= nil, "highlight group " .. group .. " should be defined")
  end
end)

h.test("state: namespaces are unique and stable", function()
  local state = require("typedown.render.state")

  local ns1 = state.get_namespace("headings")
  local ns2 = state.get_namespace("containers")
  local ns3 = state.get_namespace("headings")

  h.assert(ns1 ~= ns2, "different renderer names should get different namespaces")
  h.assert_eq(ns1, ns3, "same renderer name should return same namespace")
end)

h.test("state: cursor line tracking", function()
  local state = require("typedown.render.state")
  local buf = vim.api.nvim_get_current_buf()

  state.set_cursor_line(buf, 0)
  h.assert_eq(state.get_cursor_line(buf), 0, "cursor should be at line 0")

  state.set_cursor_line(buf, 42)
  h.assert_eq(state.get_cursor_line(buf), 42, "cursor should be at line 42")
end)

h.test("state: attach and detach", function()
  local state = require("typedown.render.state")
  local fake_buf = 99999

  h.assert(not state.is_attached(fake_buf), "fresh buffer should not be attached")
  state.mark_attached(fake_buf)
  h.assert(state.is_attached(fake_buf), "marked buffer should be attached")
  state.detach(fake_buf)
  h.assert(not state.is_attached(fake_buf), "detached buffer should not be attached")
end)

h.test("anti-conceal: cursor move triggers re-render flag", function()
  local state = require("typedown.render.state")
  local anti_conceal = require("typedown.render.anti-conceal")
  local buf = vim.api.nvim_get_current_buf()

  state.set_cursor_line(buf, 5)
  vim.api.nvim_win_set_cursor(0, { 1, 0 })

  local needs_render = anti_conceal.on_cursor_moved(buf)
  h.assert(needs_render, "moving to a different line should trigger re-render")

  local needs_render2 = anti_conceal.on_cursor_moved(buf)
  h.assert(not needs_render2, "staying on same line should not trigger re-render")
end)

h.test("anti-conceal: insert mode sets special cursor value", function()
  local state = require("typedown.render.state")
  local anti_conceal = require("typedown.render.anti-conceal")
  local buf = vim.api.nvim_get_current_buf()

  state.set_cursor_line(buf, 5)
  anti_conceal.on_insert_enter(buf)
  h.assert_eq(state.get_cursor_line(buf), -2, "insert mode should set cursor to -2 (all lines raw)")

  anti_conceal.on_insert_leave(buf)
  h.assert(state.get_cursor_line(buf) ~= -2, "leaving insert should restore cursor line")
end)

h.test("container: both opening and closing ::: are rendered", function()
  local lang_ok = pcall(vim.treesitter.language.inspect, "typedown_md")
  if not lang_ok then
    io.write("  (skipped: typedown_md parser not available)\n")
    return
  end

  vim.cmd("enew")
  vim.bo.filetype = "typedown"
  local buf = vim.api.nvim_get_current_buf()
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, {
    "---", "---",
    "::: note",
    "content",
    ":::",
  })

  -- Move cursor away from both delimiters
  vim.api.nvim_win_set_cursor(0, { 4, 0 })

  -- Set cursor line in state so is_hidden works correctly
  local render_state = require("typedown.render.state")
  render_state.set_cursor_line(buf, 3) -- 0-indexed line 3 = row 4

  -- Render directly (attach defers, which doesn't work in headless)
  local render = require("typedown.render.init")
  render.render_buf(buf)

  local ns = require("typedown.render.state").get_namespace("containers")
  local marks = vim.api.nvim_buf_get_extmarks(buf, ns, 0, -1, { details = true })

  local has_opening_overlay = false
  local has_closing_overlay = false
  for _, mark in ipairs(marks) do
    local row = mark[2]
    local details = mark[4]
    if details.virt_text and details.virt_text_pos == "overlay" then
      if row == 2 then has_opening_overlay = true end
      if row == 4 then has_closing_overlay = true end
    end
  end

  h.assert(has_opening_overlay, "opening ::: (row 2) should have an overlay extmark")
  h.assert(has_closing_overlay, "closing ::: (row 4) should have an overlay extmark")
end)

h.test("typedown_yaml highlights query loads without errors", function()
  local lang_ok = pcall(vim.treesitter.language.inspect, "typedown_yaml")
  if not lang_ok then
    io.write("  (skipped: typedown_yaml parser not available)\n")
    return
  end
  local ok, err = pcall(vim.treesitter.query.get, "typedown_yaml", "highlights")
  h.assert(ok, "typedown_yaml highlights query should load without errors: " .. tostring(err))
end)

h.test("inline formatting highlights are correctly set", function()
  require("typedown.render.highlights").setup()

  local strong = vim.api.nvim_get_hl(0, { name = "@markup.strong" })
  h.assert(strong.bold, "@markup.strong should be bold")

  local italic = vim.api.nvim_get_hl(0, { name = "@markup.italic" })
  h.assert(italic.italic, "@markup.italic should be italic")

  local strike = vim.api.nvim_get_hl(0, { name = "@markup.strikethrough" })
  h.assert(strike.strikethrough, "@markup.strikethrough should be strikethrough")
end)

h.test("all renderers have a name field", function()
  local renderers = {
    require("typedown.render.headings"),
    require("typedown.render.containers"),
    require("typedown.render.checkboxes"),
    require("typedown.render.bullets"),
    require("typedown.render.code"),
    require("typedown.render.tables"),
    require("typedown.render.toggle-list"),
  }
  for _, renderer in ipairs(renderers) do
    h.assert(renderer.name, "renderer should have a name field")
    h.assert(type(renderer.name) == "string", "renderer.name should be a string")
  end
end)

h.test("multi-state checkbox toggle cycles through states", function()
  vim.cmd("enew")
  vim.bo.filetype = "typedown"
  local buf = vim.api.nvim_get_current_buf()
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, {
    "---", "---",
    "- [ ] unchecked",
  })

  local checkboxes = require("typedown.render.checkboxes")

  -- [ ] -> [x]
  vim.api.nvim_win_set_cursor(0, { 3, 0 })
  checkboxes.toggle()
  local line = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1]
  h.assert_eq(line, "- [x] unchecked", "first toggle: [ ] -> [x]")

  -- [x] -> [~]
  checkboxes.toggle()
  line = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1]
  h.assert_eq(line, "- [~] unchecked", "second toggle: [x] -> [~]")

  -- [~] -> [>]
  checkboxes.toggle()
  line = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1]
  h.assert_eq(line, "- [>] unchecked", "third toggle: [~] -> [>]")

  -- [>] -> [!]
  checkboxes.toggle()
  line = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1]
  h.assert_eq(line, "- [!] unchecked", "fourth toggle: [>] -> [!]")

  -- [!] -> [ ]
  checkboxes.toggle()
  line = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1]
  h.assert_eq(line, "- [ ] unchecked", "fifth toggle: [!] -> [ ] (full cycle)")
end)

h.test("state: is_hidden respects anti-conceal range", function()
  local render_state = require("typedown.render.state")
  local buf = vim.api.nvim_get_current_buf()

  render_state.set_cursor_line(buf, 10)

  -- Default range (above=0, below=0): only cursor line is hidden
  h.assert(render_state.is_hidden(buf, 10), "cursor line should be hidden")
  h.assert(not render_state.is_hidden(buf, 9), "line above should not be hidden")
  h.assert(not render_state.is_hidden(buf, 11), "line below should not be hidden")

  -- Expand range
  local orig_above = render_state.anti_conceal.above
  local orig_below = render_state.anti_conceal.below
  render_state.anti_conceal.above = 2
  render_state.anti_conceal.below = 1

  h.assert(render_state.is_hidden(buf, 8), "2 lines above should be hidden")
  h.assert(render_state.is_hidden(buf, 9), "1 line above should be hidden")
  h.assert(render_state.is_hidden(buf, 10), "cursor line should be hidden")
  h.assert(render_state.is_hidden(buf, 11), "1 line below should be hidden")
  h.assert(not render_state.is_hidden(buf, 12), "2 lines below should not be hidden")
  h.assert(not render_state.is_hidden(buf, 7), "3 lines above should not be hidden")

  -- Restore
  render_state.anti_conceal.above = orig_above
  render_state.anti_conceal.below = orig_below
end)

h.test("state: mode tracking", function()
  local render_state = require("typedown.render.state")
  local buf = vim.api.nvim_get_current_buf()

  render_state.set_mode(buf, "n")
  h.assert(not render_state.is_mode_disabled(buf), "normal mode should not be disabled")

  render_state.set_mode(buf, "i")
  h.assert(render_state.is_mode_disabled(buf), "insert mode should be disabled")

  render_state.set_mode(buf, "v")
  h.assert(not render_state.is_mode_disabled(buf), "visual mode should not be disabled")

  render_state.set_mode(buf, "n")
end)

h.test("state: changedtick tracking", function()
  local render_state = require("typedown.render.state")
  vim.cmd("enew")
  local buf = vim.api.nvim_get_current_buf()

  -- First call should always return true (tick mismatch)
  h.assert(render_state.has_changed(buf), "first has_changed should be true")

  -- Second call without changes should return false
  h.assert(not render_state.has_changed(buf), "second has_changed should be false (no change)")

  -- Modify buffer
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "changed" })
  h.assert(render_state.has_changed(buf), "has_changed should be true after modification")
end)

h.test("links renderer has a name field", function()
  local links = require("typedown.render.links")
  h.assert_eq(links.name, "links", "links renderer should have name 'links'")
end)

h.test("smart-action module loads without error", function()
  local ok, err = pcall(require, "typedown.render.smart-action")
  h.assert(ok, "smart-action should load without error: " .. tostring(err))
end)

h.test("commands are registered", function()
  require("typedown.render.checkboxes").setup_commands()
  require("typedown.render.toggle-list").setup_commands()

  -- Check commands exist
  local commands = vim.api.nvim_get_commands({})
  h.assert(commands["TypedownToggleCheckbox"], "TypedownToggleCheckbox command should exist")
  h.assert(commands["TypedownToggle"], "TypedownToggle command should exist")
end)

h.finish()
