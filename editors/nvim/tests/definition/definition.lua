local h = require("tests.helper")

io.write("Go to Definition:\n")

h.test("_type: Person resolves to _types/Person.td", function()
  h.open_fixture("basic-vault", "alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- Position on "Person" at line 1, character 7
  local params = vim.lsp.util.make_position_params(0, client.offset_encoding)
  params.position = { line = 1, character = 7 }

  local result = h.lsp_request("textDocument/definition", params)
  h.assert(result, "should return definition result")

  -- Result can be Location or Location[]
  local locations = vim.islist(result) and result or { result }
  h.assert(#locations > 0, "should resolve to at least one definition")
  h.assert(#locations == 1, "should resolve to exactly one definition, got " .. #locations)

  local uri = locations[1].uri or locations[1].targetUri
  h.assert(uri:find("Person%.td"), "should point to Person.td, got: " .. uri)
end)

h.test("fref resolves to referenced file", function()
  h.open_fixture("basic-vault", "with-fref.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- Position on "alice.td" inside fref("alice.td") at line 4
  local params = vim.lsp.util.make_position_params(0, client.offset_encoding)
  params.position = { line = 4, character = 18 }

  local result = h.lsp_request("textDocument/definition", params)
  h.assert(result, "should return definition result for fref")

  local locations = vim.islist(result) and result or { result }
  h.assert(#locations > 0, "should resolve fref to a definition")

  local uri = locations[1].uri or locations[1].targetUri
  h.assert(uri:find("alice%.td"), "should point to alice.td, got: " .. uri)
  h.assert(not uri:find("with%-fref"), "should NOT point to with-fref.td itself")
end)

h.test("no definition on plain string value", function()
  h.open_fixture("basic-vault", "alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- "Alice" at line 2, char 7 - plain string, not a reference
  local params = vim.lsp.util.make_position_params(0, client.offset_encoding)
  params.position = { line = 2, character = 7 }

  local result = h.lsp_request("textDocument/definition", params)
  local count = 0
  if result then
    count = vim.islist(result) and #result or 1
  end
  h.assert_eq(count, 0, "plain string should not resolve to a definition")
end)

h.finish()
