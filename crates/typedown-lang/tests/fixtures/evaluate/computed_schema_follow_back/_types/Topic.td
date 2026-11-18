---
_type: schema
properties:
  name: string
  articles:
    type: list[Article]
    computed: (r) -> vault.from_node(r).follow_back(Article, "topics").to_list()
---
