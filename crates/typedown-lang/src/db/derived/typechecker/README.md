# Typechecking algorithm (Bidirectional typechecking)
 
I wrote this when something clicked, suddenly.

Previously, I had some vague idea about bidirectional typechecking:
   1. You start from the top of the expression with the expected type and infer down the expected type to the child expressions
   2. You can also start from the atomic expressions like constants to get their actual types, then use rules to infer up the actual type
   3. Any mismatches between actual types and expected types are reported.
So basically, it's like you running in parallel two lines of inference and try to reconcile them.
 
However, this isn't always that clear cut between the two lines... Sometimes, they have to overlap, i.e to infer expected type, we may need to infer down from an "actual type".

For example, try typechecking this:

```ts
let a: number = ((x) => x + x)(3)
```

To infer `(x) => x + x`'s expected type, you have to get actual type of the arg.

To get actual type of `((x) => x + x)(3)`, you have to get expected type of `(x) => x + x`.

So... the question is, if they can call each other, how should we be sure there isn't an infinite loop.

This is when I refine my idea like so:
1. Define handoff points: In which kind of expressions will expected type just retrieve the actual type and vice versa?
2. When perform inferring expected type, you should stop a branch when it reach the handoff point for the actual type and vice versa.

When defining handoff points, you should beware of circularity.
