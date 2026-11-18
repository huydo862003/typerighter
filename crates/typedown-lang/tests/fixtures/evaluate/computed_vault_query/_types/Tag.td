---
_type: schema
properties:
  label: string
  books:
    type: list[Book]
    computed: (r) -> vault.from_node(r).follow_back(Book, "tag").to_list()
  bookCount:
    type: number
    computed: (r) -> vault.from_node(r).follow_back(Book, "tag").count()
  textbooks:
    type: list[Textbook]
    computed: (r) -> vault.from_node(r).follow_back(Textbook, "tag").to_list()
---
