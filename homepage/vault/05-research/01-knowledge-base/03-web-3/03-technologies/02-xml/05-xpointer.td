---
_label: "XPointer: Fragment Identifiers for XML"
_icon: icon.document
---

![XSL Family](../../../../assets/xsl-family.svg)

XPointer allows links to point to **specific parts** of an XML document, not just the whole document. It uses XPath expressions to identify the target. Used with [XLink](./xlink.md) to link to fragments of remote XML documents.

> Remark: No browser supports XPointer, however, it is used in other XML languages.

## Syntax

The general form is a URL followed by a `#` fragment identifier:

```
url#pointer
```

The `pointer` part has 2 forms:

1. Targets an element by its ID directly.

   ```
   dogbreeds.xml#Rottweiler
   ```

2. Scheme-based: Wraps an XPath expression in `xpointer(...)`.

   ```
   dogbreeds.xml#xpointer(id('Rottweiler'))
   dogbreeds.xml#xpointer(/dogbreeds/dog[1])
   ```

Note that `id()` is an XPath function that matches attributes declared as type `ID` in a DTD or `xml:id` attributes, not just any attribute named "id".

Multiple fragments can be provided: The schemes are evaluated left to right, the first match wins.

```
dogbreeds.xml#Rottweiler#xpointer(/dogbreeds/dog[1])
```

The context node in XPointer is always the document root, so relative XPath expressions evaluate from the root and are effectively the same as absolute ones.

## Example

Target document (`dogbreeds.xml`):

```xml
<dogbreeds>
  <dog breed="Rottweiler" id="Rottweiler">
    <history>The Rottweiler's ancestors were...</history>
  </dog>
  <dog breed="FCRetriever" id="FCRetriever">
    <history>One of the earliest uses of...</history>
  </dog>
</dogbreeds>
```

Linking document:

```xml
<mydogs xmlns:xlink="http://www.w3.org/1999/xlink">
  <mydog>
    <fact xlink:type="simple" xlink:href="dogbreeds.xml#Rottweiler">
      Fact about Rottweiler
    </fact>
  </mydog>
</mydogs>
```

> Remark: XPointer is to XML what `#fragment` is to HTML, but it is more powerful because it uses XPath, so we can target any node, not just elements with `id` attributes.
