---
title: "Component Demo"
---

## Alerts

Alert with a `kind` prop:

:::td-alert{kind="info"}
This is an alert component.
:::

Alert without props:

:::td-alert
Alert without props.
:::

## Data table

Striped table with a caption:

:::td-data-table{caption="Monthly sales" striped="true"}
| month | revenue |
| ----- | ------- |
| jan   | \$1,200  |
| feb   | \$980    |
| mar   | \$1,540  |
:::

## Tabs

Tabs with three labels and a default active tab:

:::td-tabs{labels="Overview, Details, Notes" active="1"}
This is the tab panel content.
It contains **bold text** and a list:

- item one
- item two
:::

## Card

Card with icon, title, body, and footer:

:::td-card{title="Getting started" icon="🚀" footer="Last updated: today"}
Follow these steps to get going:

1. Install the package
2. Configure your vault
3. Run the dev server
:::
