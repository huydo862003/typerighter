---
_label: "XML Core"
_icon: icon.document
---

XML stands for eXtensible Markup Language, designed to store and transport data & self-descriptive.

W3C recommends XML.

> Remark: I find XML to be verbose and user-unfriendly though. But maybe it's one of the first universal and portable data transport format.

## XML vs HTML

XML vs HTML:

- XML carries **data** (focus on _what_ data is). Tags are **not** predefined, you define your own.
- HTML displays **data** (focus on _how_ data looks). Tags are predefined (`<p>`, `<div>`, etc.).

## Properties of XML

XML is **extensible**: Applications that parse XML can handle new or removed tags gracefully. An app designed for `<to> <from> <heading> <body>` can still work when it encounters a document with an added `<date>` or a removed `<heading>`. The parser just ignores what it doesn't recognize.

XML stores data in **plain text**, making it software- and hardware-independent. This simplifies data sharing, transport, and platform migration. Data isn't locked to a specific system.

## Syntax

The basic components are just like those in HTML.

### Basics

XML documents form a **tree** starting from a root element. Elements can have child elements, text content, and **attributes**.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<bookstore>
  <book category="cooking">
    <title lang="en">Everyday Italian</title>
    <author>Giada De Laurentiis</author>
    <year>2005</year>
    <price>30.00</price>
  </book>
</bookstore>
```

- `<?xml ... ?>`: Prolog. Declares XML version and character encoding.
- `<bookstore>`: Root element.
- `<book category="cooking">`: Element with an **attribute** (`category`).
- `<title>`, `<author>`, etc.: Child elements with **text content**.

The syntax is **self-describing**: You can infer the data structure just by reading the tags.

**Empty elements**: `<element></element>` and `<element />` are always identical in XML. Any element can self-close. (In HTML, self-closing only works for void elements like `<br>`, `<img>`. `<div />` is parsed as just `<div>` by HTML parsers.)

### Rules

Some differ from HTML.

- Each XML document must have exactly **one root element**, like `<html>` in HTML.
- Every opening tag **must** be closed (`<tag>...</tag>` or self-closing `<tag/>`). Tags must be **properly nested** (no overlapping).
- Attributes **must** be quoted, either in single quotes or double quotes: `category="cooking"`.
- Element naming rules are as follows.
  - Element names are **case sensitive**: `<Tag>` and `<tag>` are different.
  - Element names can only start with a letter or underscore, much like an identifier in most programming languages. Can only contain letters, digits, hyphens, underscores, and periods.
  - Element names cannot start with `xml` or any of the case variants.

A document following all these rules is called **"well-formed"**. A **"valid"** document is well-formed and additionally conforms to a document type definition, either a **DTD** or an **XML Schema**. These define the legal elements, attributes, and structure of a document.

Unlike HTML, XML has **strict error handling**: The W3C spec requires a parser to stop processing if it encounters a malformed document. HTML browsers tolerate errors (missing end tags, etc.), but XML parsers must not.

Notes:

- Whitespace in elements is **preserved** (**unlike HTML**).
- Newlines are stored as **LF** (not CR+LF).
- Unlike HTML, XML has no built-in `id` attribute. An attribute only gains ID semantics when declared as type `ID` in a DTD or when using `xml:id`.

### Namespaces

Problem: Different XML vocabularies can use the same tag name. A `<title>` in a book catalog means something different from a `<title>` in an HTML page. When you combine documents, names collide.

Solution: **Namespaces** prefix element names with a URI to make them globally unique.

```xml
<root
  xmlns:book="http://example.org/book"
  xmlns:html="http://www.w3.org/1999/xhtml">
  <book:title>XML Explained</book:title>
  <html:title>My Page</html:title>
</root>
```

`xmlns:prefix="URI"` declares a namespace. The URI doesn't have to point to anything, it's just a **unique identifier**. A namespace declaration is available on the **same element** where it's declared (not just children). XML processes namespace declarations _before_ resolving element names. For example:

```xml
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
```

Here `xsl:` is both declared and used on the same element. This is valid because the parser knows what `xsl:` means by the time it reads the element name.

A namespace declaration applies from the element it's declared on through all its children, unless overridden.

A default namespaces can be declared by specifying `xmlns` alone:

```xml
<table xmlns="http://www.w3.org/TR/html4/">
  <tr>
    <td>Apples</td>
    <td>Bananas</td>
  </tr>
</table>
```

## XSL Family

XML provides the structure. XSL allows querying & transforming XML.

![XSL Family](../../../../assets/xsl-family.svg)

**XPath** is at the center: All the others depend on it for navigating XML.

**XSL** (eXtensible Stylesheet Language) is specifically about **styling and transforming** XML, the XML counterpart to what CSS does for HTML. The W3C defines it as three parts:

- **XSLT**: Transforming XML documents. See [XSLT](./xslt.md).
- **XPath**: Navigating XML documents. See [XPath](./xpath.md).
- **XSL-FO**: Formatting XML (discontinued 2013, replaced by CSS3 Paged Media).

**XQuery** is commonly grouped with XSL because it shares XPath and is about querying XML, but it is technically a separate W3C spec. See [XQuery](./xquery.md).

Also in the diagram but not part of XSL:

- **XPointer**: Points to **specific parts** of an XML document (not just the whole document). Uses XPath expressions after a `#` fragment: `dogbreeds.xml#xpointer(id('Rottweiler'))` or shorthand `dogbreeds.xml#Rottweiler`. Used with XLink to link to fragments of remote XML documents. No browser support.
- **XLink**: Hyperlinks for XML. In HTML, `<a>` is the link element. In XML, tags are user-defined so there's no predefined link element. XLink adds linking via namespace attributes (`xlink:type`, `xlink:href`) on **any** element. Supports simple links (like HTML) and extended links (links defined outside the linked files, embedded resources, etc.). No browser support for XML documents, but supported in SVG.

XLink and XPointer are not part of XSL because they are about **linking and addressing**, not styling or transforming. They were developed by a different W3C working group (XML Linking Working Group). They appear in the same diagram because they all share XPath as a dependency.

These three form a layered dependency: XPath provides the query language to select nodes, XPointer builds on XPath to identify specific document fragments, and XLink uses XPointer in its `href` attributes to link to those fragments.
