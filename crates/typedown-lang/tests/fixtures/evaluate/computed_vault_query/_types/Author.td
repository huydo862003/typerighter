---
_type: schema
properties:
  name: string
  books:
    type: list[Book]
    computed: (r) -> vault.from_node(r).follow_back(Book, "author").to_list()
---
