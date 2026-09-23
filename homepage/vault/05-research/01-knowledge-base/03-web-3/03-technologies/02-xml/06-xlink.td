---
_label: "XLink: Hyperlinks for XML"
_icon: icon.document
---

![XSL Family](../../../../assets/xsl-family.svg)

XLink creates hyperlinks in XML documents.

In HTML, `<a>` is the link element. In XML, tags are user-defined, so there is no predefined link element. XLink solves this by adding linking via **namespace attributes** on any element.

Interesting observation: There is no browser support for XLink in XML documents, but all major browsers support **XLink in SVG**.

## How It Works

You declare the XLink namespace, then add `xlink:type` and `xlink:href` to any element.

```xml
<homepages xmlns:xlink="http://www.w3.org/1999/xlink">
  <homepage xlink:type="simple" xlink:href="https://example.com">Visit Example</homepage>
</homepages>
```

Any element can be a link, and the element name does not matter.

## Attributes

| Attribute       | Values                                                      | Purpose                                                                                                                             |
| --------------- | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `xlink:type`    | `simple`, `extended`, `locator`, `arc`, `resource`, `title` | Specifies the type of link.                                                                                                         |
| `xlink:href`    | URL                                                         | Specifies where to link to.                                                                                                         |
| `xlink:show`    | `embed`, `new`, `replace`                                   | Controls how to display the linked resource. `embed` inlines it, `new` opens a new window, and `replace` replaces the current view. |
| `xlink:actuate` | `onLoad`, `onRequest`                                       | Controls when to load the resource. `onLoad` loads immediately, and `onRequest` waits for user action.                              |

## Simple Links

A simple link works like HTML's `<a href="...">`. It has one source and one destination.

```xml
<homepage xlink:type="simple" xlink:href="https://example.com">Visit</homepage>
```

With `xlink:show="embed"`, the linked resource is **inlined** into the page, similar to an iframe. When combined with `xlink:actuate="onLoad"`, it loads automatically. This means you could build a hierarchy of XML documents by embedding them into each other.

## Extended Links

Where a simple link is like an HTML `<a href="...">` (one source, one destination, all in one element), an extended link is like a **routing table**. Instead of hardcoding "A links to B", you declare resources and then declare how to travel between them.

An extended link has four sub-element types:

- **`locator`** (`xlink:type="locator"`) is a **remote resource**. It points to an external document or node via `xlink:href` and is labeled with `xlink:label`.
- **`resource`** (`xlink:type="resource"`) is a **local resource**. It represents content that is inline within the link element itself, the visible text in the current document. It is also labeled.
- **`arc`** (`xlink:type="arc"`) is a **traversal rule** from one label to another, defined by `xlink:from` and `xlink:to`. Arcs connect labels, not elements directly, so one arc can apply to multiple resources sharing the same label.
- **`title`** (`xlink:type="title"`) provides a human-readable label for the link.

```xml
<glossary xlink:type="extended" xmlns:xlink="http://www.w3.org/1999/xlink">
  <!-- two remote resources -->
  <loc xlink:type="locator" xlink:href="uri-spec.xml" xlink:label="spec"/>
  <loc xlink:type="locator" xlink:href="uri-tutorial.xml" xlink:label="tutorial"/>

  <!-- one local resource (visible anchor in the document) -->
  <term xlink:type="resource" xlink:label="term">URI</term>

  <!-- arcs: from "term" to "spec" and "tutorial" -->
  <go xlink:type="arc" xlink:from="term" xlink:to="spec" xlink:show="new"/>
  <go xlink:type="arc" xlink:from="term" xlink:to="tutorial" xlink:show="embed"/>
</glossary>
```

The word "URI" is a local resource, the visible anchor. The spec and tutorial are remote resources, referenced by URL. The arcs define that from "URI", you can navigate to the spec (opens in a new window) or the tutorial (embedded inline). Each destination has different display behavior.

The distinction between local and remote resources determines what kinds of links are possible:

- **Local to remote** works like a traditional hyperlink. The user sees the local content and can navigate to an external resource.
- **Remote to remote** is a **third-party link**. The extended link connects two external documents, and neither of them needs to know about the link. The link is defined in a separate document from the resources it connects.

Without local resources, an extended link is purely a third-party routing table with no visible anchor in the current document.

> Remark: Extended links separate the _what_ (locators and resources), the _where_ (arcs), and the _how_ (show/actuate) of linking. Simple links bundle all three into one element.

> Remark: Extended links decouple the link definition from the resources. This means links can be inline (the local resource is the anchor, arcs point outward), out-of-line (the link element lives in a separate document from the resources it connects), or retroactive (you can link existing documents without modifying them by creating an extended link elsewhere that references them as locators). This can be useful for Typedown.
