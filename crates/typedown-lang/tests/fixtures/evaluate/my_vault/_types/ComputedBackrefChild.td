---
_type: schema
properties:
  name: string
  parent: ComputedBackrefParent
  siblings:
    type: list[ComputedBackrefChild]
    computed: (r) -> vault.from_node(r.parent).follow_back(ComputedBackrefChild, "parent").to_list()
---
