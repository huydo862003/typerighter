---
_type: schema
properties:
  a:
    type: number
  bad:
    type: number
    computed: (r) -> r.nonexistent
---
