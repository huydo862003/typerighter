local h = require("tests.helper")

io.write("Rename:\n")

h.test("prepare rename on _type value returns range and placeholder", function()
  h.open_fixture("rename-vault", "alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- Position on "Person" in `_type: Person` (line 1, char 7)
  local params = vim.lsp.util.make_position_params(0, client.offset_encoding)
  params.position = { line = 1, character = 7 }

  local result = h.lsp_request("textDocument/prepareRename", params)
  h.assert(result, "prepareRename should return a result for _type reference")

  -- Check the range covers "Person"
  local range = result.range or result
  h.assert_eq(range.start.line, 1, "range should start on line 1")
  h.assert_eq(range.start.character, 7, "range should start at character 7")
  h.assert_eq(range["end"].character, 13, "range should end at character 13")

  -- Check placeholder
  if result.placeholder then
    h.assert_eq(result.placeholder, "Person", "placeholder should be 'Person'")
  end
end)

h.test("prepare rename on plain string value returns null", function()
  h.open_fixture("rename-vault", "alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- Position on "Alice" at line 2, char 7
  local params = vim.lsp.util.make_position_params(0, client.offset_encoding)
  params.position = { line = 2, character = 7 }

  local result, err = h.lsp_request("textDocument/prepareRename", params)
  h.assert(not result or err, "plain string value should not be renameable")
end)

h.test("rename _type: apply edit and verify file content changes", function()
  -- Copy fixture to temp so we can mutate
  local temp = h.copy_fixture("rename-vault")

  -- Open the temp copy
  h.open_temp(temp, "alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- Get rename edit for Person -> Human
  local params = vim.lsp.util.make_position_params(0, client.offset_encoding)
  params.position = { line = 1, character = 7 }
  params.newName = "Human"

  local result = h.lsp_request("textDocument/rename", params)
  h.assert(result, "rename should produce a WorkspaceEdit")

  -- Apply the workspace edit
  vim.lsp.util.apply_workspace_edit(result, client.offset_encoding)

  -- Save all modified buffers
  vim.cmd("wall")
  vim.wait(500, function() return false end, 50)

  -- Verify alice.td now has _type: Human
  local alice_content = h.read_temp_file(temp, "alice.td")
  h.assert(
    alice_content:find("_type: Human"),
    "alice.td should contain '_type: Human' after rename, got:\n" .. alice_content
  )
  h.assert(
    not alice_content:find("_type: Person"),
    "alice.td should NOT contain '_type: Person' after rename"
  )

  -- Verify refers-alice.td also changed
  local refers_content = h.read_temp_file(temp, "refers-alice.td")
  h.assert(
    refers_content:find("_type: Human"),
    "refers-alice.td should contain '_type: Human' after rename, got:\n" .. refers_content
  )

  -- Verify other fields are preserved
  h.assert(alice_content:find('name: "Alice"'), "alice.td should preserve name field")
  h.assert(alice_content:find("age: 30"), "alice.td should preserve age field")
  h.assert(refers_content:find('name: "Bob"'), "refers-alice.td should preserve name field")
  h.assert(refers_content:find('fref%('), "refers-alice.td should preserve fref call")

  h.rm_temp_vault(temp)
end)

h.test("rename fref: apply edit and verify path changes", function()
  local temp = h.copy_fixture("rename-vault")

  h.open_temp(temp, "refers-alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- Position on "alice.td" inside fref("alice.td") at line 4
  local params = vim.lsp.util.make_position_params(0, client.offset_encoding)
  params.position = { line = 4, character = 14 }

  -- Check if prepareRename works at this position
  local prepare_params = vim.lsp.util.make_position_params(0, client.offset_encoding)
  prepare_params.position = { line = 4, character = 14 }
  local prepare = h.lsp_request("textDocument/prepareRename", prepare_params)

  if prepare then
    params.newName = "bob.td"
    local result = h.lsp_request("textDocument/rename", params)
    h.assert(result, "rename on fref path should produce edits")

    -- Apply the workspace edit
    vim.lsp.util.apply_workspace_edit(result, client.offset_encoding)
    vim.cmd("wall")
    vim.wait(500, function() return false end, 50)

    -- Verify the fref path changed
    local content = h.read_temp_file(temp, "refers-alice.td")
    h.assert(
      content:find('fref%(.*bob'),
      "refers-alice.td should reference 'bob' after rename, got:\n" .. content
    )
    h.assert(
      not content:find('fref%("alice%.td"%)'),
      "refers-alice.td should NOT reference 'alice.td' after rename"
    )

    -- Verify non-fref content is preserved
    h.assert(content:find('name: "Bob"'), "refers-alice.td should preserve name field")
    h.assert(content:find("age: 25"), "refers-alice.td should preserve age field")
  end

  h.rm_temp_vault(temp)
end)

h.finish()
