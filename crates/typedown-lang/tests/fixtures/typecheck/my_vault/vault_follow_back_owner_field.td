---
_type: Person
name: "Alice"
age: 30
result: vault.from_nodes(Person).follow_back(WithBareRef, "owner").to_list()[0].name
---
