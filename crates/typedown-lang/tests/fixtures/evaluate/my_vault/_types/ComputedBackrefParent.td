---
_type: schema
properties:
  title: string
  children:
    type: list[ComputedBackrefChild]
    computed: (r) -> vault.from_node(r).follow_back(ComputedBackrefChild, "parent").to_list()
---
