---
_type: schema
properties:
  name:
    type: string
  parent:
    type: SelfRef | null
  children:
    type: list[SelfRef] | null
---
