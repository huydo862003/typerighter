---
_type: schema
properties:
  a:
    type: number
  b:
    type: number
  sum:
    type: number
    computed: (r) -> r.a + r.b
---
