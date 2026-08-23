---
_type: schema
properties:
  title:
    type: string
  target_date:
    type: date
  status:
    type: ['planned', 'in_progress', 'completed']
  project:
    type: Project
  lead:
    type: Person?
  tags:
    type: list[string]?
---
