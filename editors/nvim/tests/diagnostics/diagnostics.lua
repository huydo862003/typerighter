local h = require("tests.helper")

io.write("Diagnostics:\n")

h.test("valid file produces zero diagnostics", function()
  h.open_fixture("diagnostics-vault", "valid.td")
  h.wait_for_lsp()
  local diags = h.wait_for_no_diagnostics()
  h.assert_eq(#diags, 0, "valid file should have zero diagnostics, got " .. #diags)
end)

h.test("unknown type produces error diagnostic", function()
  h.open_fixture("diagnostics-vault", "unknown-type.td")
  h.wait_for_lsp()
  local diags = h.wait_for_diagnostics()
  h.assert(#diags > 0, "expected at least one diagnostic for unknown type")
  h.assert_eq(diags[1].severity, vim.diagnostic.severity.ERROR, "should be error severity")
end)

h.test("unknown field produces diagnostic", function()
  h.open_fixture("diagnostics-vault", "unknown-field.td")
  h.wait_for_lsp()
  local diags = h.wait_for_diagnostics()
  h.assert(#diags > 0, "expected at least one diagnostic for unknown field")
end)

h.finish()
