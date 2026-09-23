---
_label: "Cool URIs Don't Change"
_icon: icon.document
---

Link: [Cool URIs Don't Change](https://www.w3.org/Provider/Style/URI) (Tim Berners-Lee)

Key takeaways:

> - A changed URI breaks every link, bookmark, and citation pointing to it. You never fully know who linked to your content.
> - URI instability is **not** a technology problem. It's an **organizational** problem.
> - Designing URIs means **leaving information out**. If it might change, don't put it in the URI.

## Why Care?

When a URI breaks, the damage is to the **server owner's reputation**. People lose confidence. They also can't accomplish what they came for.

> Remark: This is basically the same argument as "don't break backwards compatibility in APIs".

## The Central Argument

Servers can map any URI to any file. There is nothing about HTTP that forces URIs to change. URIs break because people encode things that change (author names, technology choices, org structure) into the URI.

## Common Excuses

Berners-Lee goes through the usual excuses and dismantles them:

- "We reorganized our website".
  - Then the old URIs were badly designed. Design the new ones to survive the _next_ redesign.
- "We had to move the files".
  - The URI space is abstract. The file system is an implementation detail. Change the server mapping, not the URI.
- "John doesn't maintain that file anymore, Jane does".
  - Why was John's name in the URI? Don't put author names in URIs.
- "URIs don't have to be persistent, that's what URNs are for".
  - Most URN schemes look like `authority + date + string`, which is basically an HTTP URI. If your org can make persistent URNs, prove it by making persistent HTTP URIs right now.
- "We have too much material to track what's outdated or confidential, so we turned it all off".
  - The problem is you never tagged your content with metadata (distribution level, creation date, expiry date). Start now.
- "We don't have the right tools".
  - The one excuse Berners-Lee actually sympathizes with. Servers don't come out of the box with persistent-URI-friendly features. But the answer is to build better tools, not accept broken URIs.

> Remark: The URN excuse is interesting. It's basically saying "we'll be disciplined _later_ with a _different_ technology". If you can't be disciplined now, why would a new scheme help?

## Designing URIs

A webmaster should allocate URIs that stand for 2 years, 20 years, 200 years.

### The Principle

Treat URIs as **abstract identifiers**, not file paths. Design the URI space independently of the file system. When files move, update the server mapping, not the URI.

### What to Include

- **Creation date**: The one thing that never changes. Good as a URI prefix.
- Exception: "Latest" pages (e.g. `/money/moneydaily/latest/`) where the concept itself is ephemeral.

### What to Exclude

Everything else:

- **Author names**: People quit, hand things off.
- **Subject/topic**: Tempting but dangerous. Topics shift, terminology evolves. Hierarchical classification is fragile because subjects relate like a web, not a tree. If you must, protect with a date prefix: `/1998/pics` means "what we meant in 1998 by pics".
- **Status** ("draft", "old", "latest"): Documents evolve through statuses.
- **Access levels** ("team", "member", "public"): Content moves between tiers. W3C learned this the hard way.
- **File extensions** (.html, .cgi, .pl).
- **Software mechanisms** ("cgi", "exec", ".pl").
- **Disk names**: Just no.
- **Org structure**: Companies reorganize.

> Remark: The topic/classification point resonates. We do this all the time in codebases too, naming folders by current team structure or feature grouping, then regretting it when things shift.

### Don't Forget the Domain Name

Same rules apply to the server name. `cgi.example.com`, `secure.example.com`, `lists.example.com` all encode implementation details. Use one domain and proxy internally. Also think about whether the domain name itself will age well.

### Examples

Bad (exposes implementation):

```
http://www.nsf.gov/cgi-bin/pubsys/browser/odbrowse.pl
http://www.nsf.gov/cgi-bin/getpub?nsf9814
```

Better (date-prefixed, mostly clean):

```
http://www.nsf.gov/pubs/1998/nsf9814/nsf9814.htm
```

Good (nothing unnecessary):

```
http://www.w3.org/1998/12/01/chairs
```
