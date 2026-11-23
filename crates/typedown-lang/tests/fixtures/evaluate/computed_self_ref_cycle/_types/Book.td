---
_type: schema
properties:
  title:
    type: string
  parent:
    type: Book | null
  children:
    type: list[Book] | null
---
