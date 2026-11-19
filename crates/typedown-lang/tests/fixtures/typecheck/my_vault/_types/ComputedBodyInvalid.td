---
_type: schema
properties:
  a:
    type: number
  label:
    type: number
    computed: (r) -> r.a + "hello"
---
