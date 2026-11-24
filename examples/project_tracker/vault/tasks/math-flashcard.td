---
_type: Task
_label: "Math flashcard test"
status: "todo"
priority: "low"
project: fref("/projects/website-redesign.td")
---

::: flashcard
What is **structural induction** on terms?

=== key
A proof technique for proving a property $P(s)$ for all terms $s$:

- **Induction on depth:** given $P(r)$ for all $r$ with $\text{depth}(r) < \text{depth}(s)$, show $P(s)$
- **Induction on size:** given $P(r)$ for all $r$ with $\text{size}(r) < \text{size}(s)$, show $P(s)$
- **Structural induction:** given $P(r)$ for all *immediate subterms* $r$ of $s$, show $P(s)$
:::

::: flashcard
What is the **evaluation relation** $t \to t'$?

=== key
The **smallest binary relation** on terms satisfying all inference rules.

$$
\frac{t_1 \to t_1'}{\text{if } t_1 \text{ then } t_2 \text{ else } t_3 \to \text{if } t_1' \text{ then } t_2 \text{ else } t_3}
$$

A statement $t \to t'$ is **derivable** iff there is a derivation tree with $t \to t'$ at its root.
:::

::: flashcard
What is a **stuck term**?

=== key
A closed term that is in **normal form** but **not a value**.

> Stuckness represents a simple notion of *run-time error*.

For example, `succ true` is stuck because no evaluation rule applies.
:::

::: flashcard{deck=["Semantics", "Type Theory"]}
What are the **three approaches** to formalizing language semantics?

=== key
1. **Operational semantics** - abstract machine with states and transitions
2. **Denotational semantics** - maps terms to mathematical objects
3. **Axiomatic semantics** - meaning defined by proof rules
:::
