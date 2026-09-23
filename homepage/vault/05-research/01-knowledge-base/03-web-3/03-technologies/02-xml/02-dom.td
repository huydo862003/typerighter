---
_label: "DOM: HTML DOM for XML"
_icon: icon.document
---

XML also defines an **XML DOM** like HTML DOM, which provides mostly the same API.

- All modern browsers have a **built-in XML parser** that converts XML text into an in-memory DOM tree.
- There are two main ways to parse.
  1. **`DOMParser`**: Parses an XML string directly.

  ```js
  const parser = new DOMParser();
  const doc = parser.parseFromString(xmlString, "text/xml");
  ```

  2. **`XMLHttpRequest`**: Fetches XML from a URL and parses it. `.responseXML` gives you the DOM tree directly.

Once parsed, the XML becomes a tree of **node objects** (elements, attributes, text). You navigate it with standard DOM methods (`.getElementsByTagName()`, `.querySelector()`, etc.).

If the XML is malformed, `DOMParser` doesn't throw. Instead it returns a document containing a `<parsererror>` element.
