---
_type: schema
properties:
  status:
    type: ['todo', 'in_progress', 'done']
  priority:
    type: ['low', 'medium', 'high']
  blocked:
    type: boolean
    optional: true
  due:
    type: date
    optional: true
  project:
    type: Project
  assignee:
    type: Person
    optional: true
---
