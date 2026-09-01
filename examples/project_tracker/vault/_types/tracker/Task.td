---
_type: schema
_icon: icon.check
properties:
  status:
    type: ['todo', 'in progress', 'done']
  priority:
    type: ['low', 'medium', 'high']
  blocked:
    type: boolean?
  due:
    type: date?
  project:
    type: Project
  assignee:
    type: Person?
  tags:
    type: list[string]?
---
