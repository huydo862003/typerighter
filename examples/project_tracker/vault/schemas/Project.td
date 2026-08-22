---
_type: schema
properties:
  name:
    type: string
  description:
    type: string?
  status:
    type: ['planning', 'active', 'done']
  started:
    type: date
  archived:
    type: boolean?
---
