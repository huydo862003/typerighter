---
_type: schema
properties:
  firstName: string
  lastName: string
  fullName:
    type: string
    computed: (r) -> "${r.firstName} ${r.lastName}"
---
