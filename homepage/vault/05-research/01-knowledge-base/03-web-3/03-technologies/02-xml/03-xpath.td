---
_label: "XPath: CSS Selectors for XML"
_icon: icon.document
---

![XSL Family](../../../../assets/xsl-family.svg)

XPath (**XML Path Language**) selects nodes in an XML document using **path expressions** (like file system paths).

XPath is used in XSLT, XQuery, XPointer, XLink.

> Analogy: XPath is to XML what **CSS selectors** are to HTML. Both select nodes from a tree.

> Remark: XML starts to look like Typedown's idea. XPath is the basis for querying.

## Syntax

Two kinds of paths:

- **Absolute**: `/root/child` starts from the document root.
- **Relative**: `child/sub` starts from the current node (context node). Common inside XSLT's `for-each`.

| Symbol          | Meaning                         |
| --------------- | ------------------------------- |
| `/`             | Navigate down one level         |
| `//`            | Select descendants at any depth |
| `.`             | Current node                    |
| `..`            | Parent node                     |
| `[n]`           | Position filter (1-indexed)     |
| `[last()]`      | Last element                    |
| `[@attr]`       | Has attribute                   |
| `[@attr='val']` | Attribute equals value          |
| `[child>val]`   | Child element value comparison  |

## Examples

| Expression                  | Selects                                    |
| --------------------------- | ------------------------------------------ |
| `/root/child`               | Direct children named `child` under `root` |
| `/root/child[1]`            | First `child`                              |
| `/root/child[last()]`       | Last `child`                               |
| `/root/child[position()<3]` | First two `child` elements                 |
| `//element`                 | All `element` nodes, anywhere              |
| `//element[@attr]`          | All `element` nodes that have `attr`       |
| `//element[@attr='val']`    | All where `attr` = `val`                   |
| `/root/child[sub>10]`       | `child` elements where `sub` > 10          |
| `/root/child[sub>10]/name`  | `name` inside those filtered children      |

> Remark: Inside XSLT's `<xsl:for-each select="catalog/cd">`, the context node is each `<cd>`. So `<xsl:value-of select="title"/>` is a relative path selecting `<title>` child of the current `<cd>`.
