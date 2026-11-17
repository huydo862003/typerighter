---
_type: schema
properties:
  firstName:
    type: string
  lastName:
    type: string
  fullName:
    type: string
    computed: (r) -> "${r.firstName} ${r.lastName}"
---
