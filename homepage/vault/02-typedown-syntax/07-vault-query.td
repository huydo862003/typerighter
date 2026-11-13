---
_label: "Vault query"
_icon: icon.database
description: "Query documents across the vault with vault.from_nodes and vault.from_node for computed relations, back-references, and collection views."
---

Forward references come naturally in Typedown. A Task document can link to a Person with `assignee: fref("people/alice.td")`, and any field can traverse that link with `self.assignee.name`. But what about the reverse direction? If Alice wants to see every task assigned to her, she would have to manually list each one, duplicating information that already exists as forward references elsewhere. The vault query API solves this by computing relationships from the graph at build time.

## The problem: manual back-references are fragile

Consider a vault with Person and Task schemas. Each task points at its assignee:

```yaml{title=_types/Task.td}
---
_type: schema
properties:
  title:
    type: string
  assignee:
    type: Person
  status:
    type: 'open' | 'done'
---
```

```yaml{title=vault/tasks/auth.td}
---
_type: Task
title: "Implement auth"
assignee: fref("/people/alice.td")
status: "open"
---
```

Alice could list her tasks by hand, but every time a task is added or reassigned, someone must remember to update her file. Vault query eliminates this maintenance by computing the list from existing forward references.

## Querying all documents of a type

`vault.from_nodes` takes a schema name and returns a query chain representing all documents of that type. The chain is lazy - it builds up filters and traversals without doing any work until you call a terminal method like `.to_list()`, `.first()`, or `.count()`.

```yaml
# count all tasks in the vault
task_count: vault.from_nodes(Task).count()
```

The three terminal methods control how results are materialized:

```yaml
# all people as a list
everyone: vault.from_nodes(Person).to_list()

# the first person, or null if the vault has none
someone: vault.from_nodes(Person).first()

# total number of people
headcount: vault.from_nodes(Person).count()
```

You can embed a query result in the markdown body with string interpolation:

```markdown
There are ${vault.from_nodes(Person).count()} people in this vault.
```

## Filtering with predicates

The `.filter` method takes a closure and keeps only the documents where the predicate returns true. The closure receives each document as its parameter, and the chain stays lazy through the filter step.

```yaml
# all open tasks
open_tasks: vault.from_nodes(Task).filter((task) -> task.status == "open").to_list()
```

Filter and terminal methods chain freely:

```yaml
# count of open tasks
open_count: vault.from_nodes(Task).filter((task) -> task.status == "open").count()

# first open task, or null
next_task: vault.from_nodes(Task).filter((task) -> task.status == "open").first()
```

## Computing back-references with follow_back

This is the payoff. The `.follow_back` method reverses a forward reference. Starting from a set of Person documents, `.follow_back(Task, "assignee")` finds every Task whose `assignee` field points at one of those people.

```yaml{title=vault/people/alice.td}
---
_type: Person
name: "Alice"
age: 30
my_tasks: vault.from_node(self).follow_back(Task, "assignee").to_list()
---
```

Alice's `my_tasks` field is computed from the graph. When a new task sets `assignee: fref("people/alice.td")`, it appears in Alice's list automatically without anyone editing her file.

You can combine `follow_back` with `filter` to narrow the results:

```yaml
open_tasks: vault.from_node(self).follow_back(Task, "assignee").filter((task) -> task.status == "open").to_list()
```

## Following forward references across sets

The `.follow` method traverses a `fref` field from every document in the query chain to its target. If a schema has an `owner` field pointing at Person documents, `.follow("owner")` collects all the referenced owners.

```yaml
# all owners referenced by any item
owners: vault.from_nodes(WithBareRef).follow("owner").to_list()
```

You can read fields from the followed documents by chaining further:

```yaml
first_owner_name: vault.from_nodes(WithBareRef).follow("owner").first().name
```

## Querying from the current document

`vault.from_node` wraps a single document into a query chain. Pass `self` to start a traversal from the current file.

```yaml
_type: Person
name: "Alice"
age: 30
result: vault.from_node(self).first()
```

This method is most useful as the starting point for `follow_back`, as shown in the back-reference example above.

## Chaining traversals

Builder methods compose into multi-step traversals. Each step narrows or extends the query chain, and only the terminal call at the end triggers evaluation.

```yaml
# from all adults, find tasks assigned to them, count those tasks
senior_task_count: vault.from_nodes(Person).filter((person) -> person.age > 18).follow_back(Task, "assignee").count()
```

## API reference

Each section below describes one entry point or method on query chains.

### vault.from_nodes(Schema)

Returns a query chain containing every document in the vault that conforms to the given schema. The chain is lazy and performs no work until a terminal method is called.

```
vault.from_nodes(Person) -> query chain of Person
```

### vault.from_node(document)

Wraps a single document into a query chain. Pass `self` to query from the current file, or pass any document obtained from another query.

```
vault.from_node(self) -> query chain of T
```

### Builder methods

Builder methods return a new query chain without materializing results. They can be chained in any order before a terminal call.

| Method | Signature | Description |
| --- | --- | --- |
| `.filter` | `(closure) -> node_set<T>` | Keeps documents where the closure returns true |
| `.follow` | `("field") -> node_set<U>` | Traverses a `fref` field to its target documents |
| `.follow_back` | `(Schema, "field") -> node_set<U>` | Finds all documents of `Schema` whose `field` points at the current set |

### Terminal methods

Terminal methods trigger evaluation and return a concrete value.

| Method | Signature | Description |
| --- | --- | --- |
| `.to_list()` | `-> list[T]` | Returns all matching documents as a list |
| `.first()` | `-> T or null` | Returns the first matching document, or null if none exist |
| `.count()` | `-> number` | Returns the number of matching documents |
