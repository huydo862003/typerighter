---
_type: Flashcard
_label: "What is structural induction?"
deck: "Type Theory"
tags:
  - "plt"
  - "proofs"
---

:::flashcard{deck=["Type Theory", "Proofs"]}
What is **structural induction** on terms?

===key
A proof technique for proving a property $P(s)$ for all terms $s$:

- **Induction on depth:** given $P(r)$ for all $r$ with $\text{depth}(r) < \text{depth}(s)$, show $P(s)$
- **Structural induction:** given $P(r)$ for all *immediate subterms* $r$ of $s$, show $P(s)$

$$
\forall s.\; (\forall r \prec s.\; P(r)) \Rightarrow P(s)
$$
:::
