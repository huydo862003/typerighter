---
_type: schema
properties:
  parens:
    type: ('draft' | 'published' | 'archived')
  bare:
    type: 'low' | 'medium' | 'high'
  parens_optional:
    type: ('draft' | 'published' | 'archived')?
  bare_optional:
    type: 'low' | 'medium' | 'high'?
  pipe_list:
    type: list['frontend' | 'backend' | 'devops']
  pipe_list_optional:
    type: list['frontend' | 'backend' | 'devops']?
---
