---
_type: schema
properties:
  status:
    type: ['todo', 'in_progress', 'done']
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
