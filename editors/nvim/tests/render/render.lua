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

h.test("checkbox toggle: checked to unchecked", function()
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
  h.assert_eq(line, "- [ ] done task", "should toggle [x] to [ ] and preserve text")
end)

h.test("checkbox toggle: no-op on line without checkbox", function()
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
  h.assert_eq(line, "- plain item", "should not modify line without checkbox")
end)

h.test("checkbox toggle: round-trip preserves line", function()
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
  require("typedown.render.checkboxes").toggle()
  require("typedown.render.checkboxes").toggle()
  local after = vim.api.nvim_buf_get_lines(buf, 2, 3, false)[1]
  h.assert_eq(after, original, "round-trip toggle should restore original line")
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

h.test("attach sets conceallevel=2 on current window", function()
  local lang_ok = pcall(vim.treesitter.language.inspect, "typedown_md")
  if not lang_ok then
    io.write("  (skipped: typedown_md parser not available)\n")
    return
  end

  vim.cmd("enew")
  vim.bo.filetype = "typedown"
  local buf = vim.api.nvim_get_current_buf()
  local win = vim.api.nvim_get_current_win()

  vim.api.nvim_set_option_value("conceallevel", 0, { win = win })

  require("typedown.render.init").attach(buf)

  local cl = vim.api.nvim_get_option_value("conceallevel", { win = win })
  h.assert_eq(cl, 2, "attach should set conceallevel=2 so conceal extmarks hide markers")
end)

h.test("container: both opening and closing ::: are concealed", function()
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

  -- Wait for tree-sitter parse
  vim.wait(200, function() return false end, 10)

  local render = require("typedown.render.init")
  render.attach(buf)
  vim.wait(200, function() return false end, 10)

  local ns = require("typedown.render.state").get_namespace("containers")
  local marks = vim.api.nvim_buf_get_extmarks(buf, ns, 0, -1, { details = true })

  local has_opening_conceal = false
  local has_closing_conceal = false
  for _, mark in ipairs(marks) do
    local details = mark[4]
    if details.conceal == "" then
      local row = mark[2]
      if row == 2 then has_opening_conceal = true end
      if row == 4 then has_closing_conceal = true end
    end
  end

  h.assert(has_opening_conceal, "opening ::: (row 2) should have a conceal extmark")
  h.assert(has_closing_conceal, "closing ::: (row 4) should have a conceal extmark")
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

h.test("commands are registered", function()
  require("typedown.render.checkboxes").setup_commands()
  require("typedown.render.toggle-list").setup_commands()

  -- Check commands exist
  local commands = vim.api.nvim_get_commands({})
  h.assert(commands["TypedownToggleCheckbox"], "TypedownToggleCheckbox command should exist")
  h.assert(commands["TypedownToggle"], "TypedownToggle command should exist")
end)

h.finish()
