---
_label: "XQuery: SQL for XML"
_icon: icon.document
---

![XSL Family](../../../../assets/xsl-family.svg)

XQuery is a language for querying and extracting data from XML documents. It is built on [XPath](./xpath.md) expressions. XQuery and XPath share the same data model, functions, and operators.

> Analogy: XQuery is to XML what **SQL** is to databases. Both select, filter, sort, and return data. Essentially, it is like a query language for an XML document database.

> Remark: XQuery and [XSLT](./xslt.md) overlap significantly: Both are built on XPath, both can transform XML, and they share the same function library. The difference is the programming model. XSLT is **template-driven**: You declare templates that match patterns, and the processor applies them. XQuery is **expression-driven**: You write expressions that explicitly select and construct results. XSLT is better suited for document transformation (restructuring XML/HTML), while XQuery is better suited for data querying (selecting and joining across documents).

## Overview

XQuery is **expression-based**: Everything is an expression that produces a value. XQuery is **case-sensitive**.

- Variables are defined with `$` followed by a name (e.g. `$bookstore`).
- String values can use single or double quotes.
- Comments are delimited by `(:` and `:)`.

`doc()` opens an XML file and returns the document node. From there, XPath expressions navigate through the tree (e.g. `doc("books.xml")/bookstore/book/title`).

The main feature XQuery adds on top of XPath is **FLWOR** (pronounced "flower"): `for`, `let`, `where`, `order by`, `return`. It is the equivalent of SQL's `SELECT ... FROM ... WHERE ... ORDER BY`.

| Feature                | Syntax                              | Example                                             |
| ---------------------- | ----------------------------------- | --------------------------------------------------- |
| Open a document        | `doc("file.xml")`                   | Returns the document node, then navigate with XPath |
| Variables              | `$name`                             | Must be valid XML names                             |
| Comments               | `(: ... :)`                         | Can be nested                                       |
| Predicates             | `[condition]`                       | XPath predicates: `book[price<30]`                  |
| Conditionals           | `if (expr) then expr else expr`     | `else` is required, use `else ()` for empty         |
| General comparisons    | `=`, `!=`, `<`, `<=`, `>`, `>=`     | Returns true if **any** item in sequence matches    |
| Value comparisons      | `eq`, `ne`, `lt`, `le`, `gt`, `ge`  | Errors if the expression returns multiple items     |
| Construct XML          | `<tag>{expr}</tag>`                 | Curly braces embed expressions in literal XML       |
| Extract text           | `data(node)`                        | Strips the element wrapper, returns text only       |
| User-defined functions | `declare function prefix:name(...)` | Name must be prefixed (e.g. `local:`)               |

## FLWOR Expressions

FLWOR (pronounced "flower") stands for `for`, `let`, `where`, `order by`, `return`. It is what XQuery adds on top of plain XPath.

| Clause            | Purpose                                      | Notes                                                                                           |
| ----------------- | -------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `for $x in expr`  | Binds a variable to each item, iterating     | `to` generates ranges, `at` counts iterations, multiple `in` expressions produce cross products |
| `let $x := expr`  | Binds a sequence to a variable, no iteration | `let $x := (1 to 5)` binds the whole sequence                                                   |
| `where condition` | Filters nodes                                | Supports `and`/`or`: `where $x/price>30 and $x/price<100`                                       |
| `order by expr`   | Sorts the result                             | Accepts multiple keys: `order by $x/@category, $x/title`                                        |
| `return expr`     | Specifies output, evaluated once per node    | Can construct new XML with `{}`                                                                 |

## Examples

Select all titles:

```xquery
doc("books.xml")/bookstore/book/title
```

Filter, sort, and return with FLWOR:

```xquery
for $x in doc("books.xml")/bookstore/book
where $x/price>30
order by $x/title
return $x/title
```

Construct HTML from XML:

```xquery
<ul>
{
  for $x in doc("books.xml")/bookstore/book/title
  order by $x
  return <li>{data($x)}</li>
}
</ul>
```

Conditional output:

```xquery
for $x in doc("books.xml")/bookstore/book
return if ($x/@category="children")
  then <child>{data($x/title)}</child>
  else <adult>{data($x/title)}</adult>
```

User-defined function:

```xquery
declare function local:minPrice(
  $p as xs:decimal?,
  $d as xs:decimal?
) as xs:decimal?
{
  let $disc := ($p * $d) div 100
  return ($p - $disc)
};

<minPrice>{local:minPrice($book/price, $book/discount)}</minPrice>
```
