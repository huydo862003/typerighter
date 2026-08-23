---
_type: schema
properties:
  name:
    type: string
  role:
    type: ['developer', 'designer', 'manager']
  active:
    type: boolean | null
  tags:
    type: list[string] | null
  email:
    type: string | null
---
