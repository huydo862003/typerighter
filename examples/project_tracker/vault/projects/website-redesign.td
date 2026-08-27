---
_type: Project
_label: "Website Redesign"
name: "Website Redesign"
description: "Overhaul the public-facing site with a new design system."
status: "active"
started: "2026-03-01"
archived: false
members:
  - fref("people/alice.td")
  - fref("people/bob.td")
tasks:
  - fref("tasks/design-mockups.td")
  - fref("tasks/implement-auth.td")
  - fref("tasks/write-tests.td")
---

## Goals

Rebuild the public-facing site from scratch with a consistent design system.
The main focus areas are:

- Improved navigation and information architecture
- Accessible, mobile-friendly layouts
- Faster load times through static generation

## Task Status

| Task | Priority | Status |
|------|----------|--------|
| Design mockups | High | Done |
| Implement auth | High | In progress |
| Write tests | Medium | Todo |

## Milestones

1. Design sign-off, completed 2026-04-10
2. Auth merged, target 2026-07-10
3. Full test coverage, target 2026-07-25
4. Production release, target 2026-08-01

::: details Architecture overview
The site is built as a static site with selective hydration.

```
                 +-------------+
                 |   Markdown   |
                 |   content    |
                 +------+------+
                        |
                 +------v------+
                 |  Typedown    |
                 |  compiler    |
                 +------+------+
                        |
              +---------+---------+
              |                   |
       +------v------+    +------v------+
       |  HTML pages  |    |  Search     |
       |  (SSG)       |    |  index      |
       +--------------+    +-------------+
```
:::

## Performance Budget

Page load targets follow the formula:

$$T_{total} = T_{TTFB} + T_{FCP} + T_{LCP} \leq 2.5s$$

where each metric is measured at the 75th percentile.

| Metric | Target | Current |
|--------|--------|---------|
| TTFB | < 200ms | 150ms |
| FCP | < 1.0s | 0.8s |
| LCP | < 2.5s | 1.9s |

## Progress

::: tip Current status
Design mockups are complete. Authentication is currently being implemented.
Integration tests are next in the queue once auth is merged.
:::
