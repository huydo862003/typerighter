---
_type: schema
properties:
  a:
    type: number
  bad:
    type: string
    computed: (r) -> r.a.to_string(1, 2)
---
