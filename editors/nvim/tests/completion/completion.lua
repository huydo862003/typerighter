local h = require("tests.helper")

io.write("Completion:\n")

h.test("suggests schema fields after _type declaration", function()
  h.open_fixture("basic-vault", "alice.td")
  local client = h.wait_for_lsp()
  h.assert(client, "LSP should attach")

  -- Request completions at line 4, col 0 (after age: 30, before ---)
  local params = vim.lsp.util.make_position_params(0, client.offset_encoding)
  params.position = { line = 4, character = 0 }
  params.context = { triggerKind = 1 } -- Invoked

  local result = h.lsp_request("textDocument/completion", params)
  h.assert(result, "should return completion result")

  local items = result.items or result
  local labels = {}
  for _, item in ipairs(items) do
    labels[item.label] = true
  end

  -- "friend" is not yet declared in alice.td - should be suggested
  h.assert(labels["friend"], 'should suggest "friend" field')
  -- "name" and "age" are already declared - should NOT be suggested
  h.assert(not labels["name"], '"name" is already declared, should not be suggested')
  h.assert(not labels["age"], '"age" is already declared, should not be suggested')
  h.assert(not labels["_type"], '"_type" should not be suggested as field completion')
end)

h.finish()
