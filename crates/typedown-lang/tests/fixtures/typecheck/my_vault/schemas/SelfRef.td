---
_type: schema
properties:
  name:
    type: string
  parent:
    type: SelfRef
    optional: true
  children:
    type: list[SelfRef]
    optional: true
---
