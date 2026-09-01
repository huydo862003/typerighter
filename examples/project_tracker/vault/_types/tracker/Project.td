---
_type: schema
_icon: icon.briefcase
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
  members:
    type: list[Person]?
  tasks:
    type: list[Task]?
  tags:
    type: list[string]?
---
