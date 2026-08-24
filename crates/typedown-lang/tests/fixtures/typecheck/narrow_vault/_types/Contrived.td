---
_type: schema
properties:
  literal_num:
    type: 42
  literal_bool:
    type: true
  mixed:
    type: [string, 0, true]
  nested_with_optional:
    type:
      required_field:
        type: string
      optional_field:
        type: number?
---
