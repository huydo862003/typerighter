---
_type: schema
properties:
  name:
    type: string
  role:
    type: ['developer', 'designer', 'manager']
  active:
    type: boolean
    optional: true
  tags:
    type: list[string]
    optional: true
  email:
    type: string
    optional: true
  tasks:
    type: list[Task]
    optional: true
---
