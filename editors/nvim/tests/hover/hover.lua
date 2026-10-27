local h = require("tests.helper")

io.write("Hover:\n")

h.test("hover on _type value shows type information", function()
  h.open_fixture("basic-vault", "alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  local params = vim.lsp.util.make_position_params(0, client.offset_encoding)
  params.position = { line = 1, character = 7 } -- on "Person"

  local result = h.lsp_request("textDocument/hover", params)
  h.assert(result, "should return hover result for type reference")
  h.assert(result.contents, "hover should have contents")

  local value = type(result.contents) == "string" and result.contents or result.contents.value
  h.assert(value:find("Person"), 'hover should mention "Person", got: ' .. value)
end)

h.test("no hover on frontmatter delimiter", function()
  h.open_fixture("basic-vault", "alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  local params = vim.lsp.util.make_position_params(0, client.offset_encoding)
  params.position = { line = 0, character = 1 } -- on "---"

  local result = h.lsp_request("textDocument/hover", params)
  h.assert(not result or not result.contents or result.contents.value == "", "should not have hover on ---")
end)

h.finish()
