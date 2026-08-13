---
_type: Task
_label: "Design mockups for new homepage"
title: "Design mockups for new homepage"
status: "done"
priority: "high"
blocked: false
due: "2026-04-15"
project: fref("projects/website-redesign.td")
assignee: fref("people/bob.td")
---

Completed **ahead of schedule**. Bob delivered three homepage variants.
The team reviewed them in the design review on *2026-04-10* and settled on **variant B**.
~~Variant A was briefly reconsidered~~ but ultimately dropped.

## Variants Reviewed

| Variant | Layout | Verdict |
|---------|--------|---------|
| A | Single column | Rejected, too sparse |
| B | Split hero | Accepted |
| C | Grid | Rejected, too busy |

::: details Design tokens
The design system uses a constrained set of tokens:

```css
:root {
  --spacing-xs: 4px;
  --spacing-sm: 8px;
  --spacing-md: 16px;
  --spacing-lg: 24px;
  --spacing-xl: 32px;

  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
}
```
:::

## Deliverables

- Figma source file with all components
- Exported SVG assets for icons
- Annotated spec sheet for developers

## Sign-off Checklist

- [x] Homepage approved by stakeholders
- [x] Content page approved
- [x] Search results page approved
- [x] Mobile variants reviewed
- [x] Accessibility audit passed
