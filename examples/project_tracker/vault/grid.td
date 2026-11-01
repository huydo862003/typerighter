---
title: "Grid Examples"
---

## Fixed Columns

Three equal columns:

:::grid{columns=3}
  :::grid-cell
  **Alpha**

  First cell in a three-column grid.
  :::
  :::grid-cell
  **Beta**

  Second cell in a three-column grid.
  :::
  :::grid-cell
  **Gamma**

  Third cell in a three-column grid.
  :::
:::

Two equal columns:

:::grid{columns=2}
  :::grid-cell
  **Left**

  A two-column layout is useful for side-by-side comparisons.
  :::
  :::grid-cell
  **Right**

  Each cell grows to fill its column equally.
  :::
:::

## Column Span

A cell spanning two columns in a three-column grid:

:::grid{columns=3}
  :::grid-cell{col-span=2}
  **Spanning two columns**

  This cell occupies the first two columns.
  :::
  :::grid-cell
  Normal cell
  :::
  :::grid-cell
  Normal cell
  :::
  :::grid-cell
  Normal cell
  :::
:::

## Row Span

A cell spanning two rows in a two-column grid:

:::grid{columns=2}
  :::grid-cell{row-span=2}
  **Spanning two rows**

  This cell occupies the first column across two rows.
  :::
  :::grid-cell
  Row 1, column 2
  :::
  :::grid-cell
  Row 2, column 2
  :::
:::

## Combined Span

Column and row span together in a three-column grid:

:::grid{columns=3}
  :::grid-cell{col-span=2 row-span=2}
  **Large cell**

  Spans two columns and two rows. Useful for a featured item alongside smaller ones.
  :::
  :::grid-cell
  Top right
  :::
  :::grid-cell
  Bottom right
  :::
  :::grid-cell
  Bottom left
  :::
  :::grid-cell
  Bottom center
  :::
:::
