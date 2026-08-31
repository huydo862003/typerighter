File references are edges in the vault graph. They connect one document to another and let you traverse the connection to read fields from linked resources.

## Creating an edge

Use `fref("path")` to link to another file. The path is relative to the vault root.

```yaml
---
_type: Task
title: "Implement auth"
assignee: fref("people/alice.td")
---
```

This creates an edge from the Task node to the Person node. The editor resolves the link, checks that the target exists, and verifies that the target's type matches the schema's declared type.

## Typed edges

When a schema field has a schema type (like `Person`), the `fref` target must conform to that schema. This is a type-checked edge.

```yaml
# _types/Task.td
---
_type: schema
properties:
  title:
    type: string
  assignee:
    type: Person?
  subtasks:
    type: list[Task]?
---
```

The type checker rejects `assignee: fref("tasks/setup-ci.td")` because a Task is not a Person.

## Traversing edges

Dot access crosses edges. If `assignee` is a `fref` to a Person, you can read the Person's fields:

```yaml
assignee_name: self.assignee.name
assignee_email: self.assignee.email
```

Traversal chains work too:

```yaml
# If a Task has a project, and a Project has an owner:
owner_name: self.project.owner.name
```

## Edges in the markdown body

Use string interpolation to render linked data in prose:

```markdown
This task is assigned to ${self.assignee.name}.
The project lead is ${self.project.owner.name}.
```

The expressions are evaluated at build time. The rendered page shows the resolved values.

## Edges in the rendered site

In the generated site, `fref` fields render as clickable links to the target page. Lists of `fref` values render as icon-prefixed link lists in the frontmatter display.

## The content graph

Every `fref` is an edge. Every `.td` file is a node. Together they form a directed graph. The type system ensures that edges connect compatible node types, and the evaluator resolves data across edges at build time.

```
    alice.td ◄────── task-1.td ──────► launch.td
    (Person)         (Task)            (Milestone)
                       │
                       ▼
                  task-2.td
                   (Task)
```

This graph is what powers autocompletion across links, type checking of `fref` targets, and the rendered relation lists in the site.
