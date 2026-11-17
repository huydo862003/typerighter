---
_type: schema
properties:
  name:
    type: string
  personCount:
    type: number
    computed: (r) -> vault.from_nodes(Person).count()
---
