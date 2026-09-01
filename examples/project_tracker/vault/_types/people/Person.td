---
_type: schema
_icon: icon.user
properties:
  name:
    type: string
  role:
    type: ['developer', 'designer', 'manager']
  active:
    type: boolean?
  tags:
    type: list[string]?
  email:
    type: string?
  tasks:
    type: list[Task]?
---
