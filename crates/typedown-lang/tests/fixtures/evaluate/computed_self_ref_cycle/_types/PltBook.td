---
_type: schema
_extends: Book
properties:
  tags:
    type: list[string] | null
  children:
    type: list[PltBook] | null
    computed: (r) -> vault.from_node(r).follow_back(PltBook, "parent").to_list()
---
