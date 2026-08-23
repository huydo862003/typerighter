---
_type: schema
properties:
  firstName: string
  lastName: string
  fullName:
    type: string
    computed: (r) -> "Alice Smith"
---
