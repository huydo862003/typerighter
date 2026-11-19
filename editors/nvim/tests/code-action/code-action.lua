local h = require("tests.helper")

io.write("Code Action:\n")

h.test("create linked resource: end-to-end file creation and fref insertion", function()
  -- Copy fixture to temp so we can mutate
  local temp = h.copy_fixture("basic-vault")

  -- Open the file with the empty schema-typed field
  h.open_temp(temp, "empty-friend.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- Get code actions at the empty 'friend:' field (line 4)
  local params = vim.lsp.util.make_range_params(0, client.offset_encoding)
  params.range.start = { line = 4, character = 7 }
  params.range["end"] = { line = 4, character = 7 }
  params.context = { diagnostics = {} }

  local result = h.lsp_request("textDocument/codeAction", params)
  h.assert(result, "should return code actions")

  -- Find the "Create new Person" action
  local create_action = nil
  for _, action in ipairs(result) do
    if action.title and action.title:find("Create new") and action.title:find("Person") then
      create_action = action
      break
    end
  end
  h.assert(create_action, "should have a 'Create new Person' code action")
  h.assert(create_action.command, "action should have a command")
  h.assert(create_action.command.command, "command should have a command name")

  -- Verify the command has the right structure
  local args = create_action.command.arguments and create_action.command.arguments[1]
  h.assert(args, "command should have arguments")
  h.assert_eq(args.schema, "Person", "schema should be Person")
  h.assert(args.sourceUri, "should have sourceUri")
  h.assert(args.prompts, "should have prompts for filename")
  h.assert(#args.prompts > 0, "should have at least one prompt")
  h.assert_eq(args.prompts[1].field, "filename", "first prompt should ask for filename")

  -- Execute the command with a pre-filled filename (bypass the interactive prompt)
  args.filename = "new-friend.td"
  args.prompts = {} -- Clear prompts since we filled the field

  local exec_result = h.lsp_request("workspace/executeCommand", {
    command = create_action.command.command,
    arguments = { args },
  })

  -- Wait for the server to apply the workspace edit
  vim.wait(2000, function() return false end, 50)

  -- Verify the new file was created with correct frontmatter
  local new_file_exists = h.temp_file_exists(temp, "new-friend.td")
  if new_file_exists then
    local new_content = h.read_temp_file(temp, "new-friend.td")
    h.assert(new_content:find("_type: Person"), "new file should have _type: Person")
    h.assert(new_content:find("name:"), "new file should have name field from schema")
  end

  -- Verify the source file got an fref inserted
  -- Reload the buffer to see server-applied changes
  vim.cmd("checktime")
  vim.wait(500, function() return false end, 50)
  local source_content = h.read_temp_file(temp, "empty-friend.td")
  if new_file_exists then
    h.assert(
      source_content:find("fref%("),
      "source file should have fref() inserted, got:\n" .. source_content
    )
  end

  h.rm_temp_vault(temp)
end)

h.test("no initialization actions on non-empty file", function()
  h.open_fixture("basic-vault", "alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  local params = vim.lsp.util.make_range_params(0, client.offset_encoding)
  params.range.start = { line = 0, character = 0 }
  params.range["end"] = { line = 0, character = 0 }
  params.context = { diagnostics = {} }

  local result = h.lsp_request("textDocument/codeAction", params)

  if result then
    for _, action in ipairs(result) do
      if action.title then
        h.assert(
          not action.title:find("^Initialize as"),
          "non-empty file should not offer initialization: " .. action.title
        )
      end
    end
  end
end)

h.test("create linked resource in markdown body offers schema picker", function()
  h.open_fixture("basic-vault", "with-body.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- Position in the markdown body (line 5)
  local params = vim.lsp.util.make_range_params(0, client.offset_encoding)
  params.range.start = { line = 5, character = 5 }
  params.range["end"] = { line = 5, character = 5 }
  params.context = { diagnostics = {} }

  local result = h.lsp_request("textDocument/codeAction", params)
  h.assert(result, "should return code actions in markdown body")

  local body_action = nil
  for _, action in ipairs(result) do
    if action.title and action.title:find("Create new file and link here") then
      body_action = action
      break
    end
  end
  h.assert(body_action, "should have 'Create new file and link here' action in body")

  local args = body_action.command.arguments and body_action.command.arguments[1]
  h.assert(args, "command should have arguments")
  h.assert(args.prompts, "should have prompts")

  -- Should have a select prompt for schema and an input prompt for filename
  local has_select = false
  local has_input = false
  for _, prompt in ipairs(args.prompts) do
    if prompt.kind == "select" then has_select = true end
    if prompt.kind == "input" then has_input = true end
  end
  h.assert(has_select, "should have a select prompt for schema")
  h.assert(has_input, "should have an input prompt for filename")

  -- insertContext should be markdownBody
  h.assert_eq(args.insertContext.kind, "markdownBody", "insert context should be markdownBody")
end)

h.test("backward link action when referencing schema exists", function()
  h.open_fixture("basic-vault", "alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- Position in frontmatter
  local params = vim.lsp.util.make_range_params(0, client.offset_encoding)
  params.range.start = { line = 2, character = 0 }
  params.range["end"] = { line = 2, character = 0 }
  params.context = { diagnostics = {} }

  local result = h.lsp_request("textDocument/codeAction", params)
  h.assert(result, "should return code actions")

  local backward_action = nil
  for _, action in ipairs(result) do
    if action.title and action.title:find("linking back here") then
      backward_action = action
      break
    end
  end
  h.assert(backward_action, "should have backward link action (Pet references Person via owner)")

  local args = backward_action.command.arguments and backward_action.command.arguments[1]
  h.assert(args, "command should have arguments")
  h.assert(args.insertContext, "should have insertContext")
  h.assert_eq(args.insertContext.kind, "backwardLink", "insert context should be backwardLink")
  h.assert(args.insertContext.field, "should have field in backward link context")
end)

h.test("no create-linked action on string field", function()
  h.open_fixture("basic-vault", "alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- Position on 'name: "Alice"' (line 2) - string field, not schema-typed
  local params = vim.lsp.util.make_range_params(0, client.offset_encoding)
  params.range.start = { line = 2, character = 0 }
  params.range["end"] = { line = 2, character = 0 }
  params.context = { diagnostics = {} }

  local result = h.lsp_request("textDocument/codeAction", params)

  if result then
    for _, action in ipairs(result) do
      if action.title then
        h.assert(
          not action.title:find("Create new"),
          "string field should not offer create-linked: " .. action.title
        )
      end
    end
  end
end)

h.finish()
