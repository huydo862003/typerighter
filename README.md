# Typedown

![Status](https://img.shields.io/badge/status-alpha-orange)
[![npm](https://img.shields.io/npm/v/typerighter)](https://www.npmjs.com/package/typerighter)
![License](https://img.shields.io/badge/license-GPL-green)
<a href="https://github.com/huydo862003/Fck-AI-Slop#plan"><img src="https://img.shields.io/badge/Human%20slop-90EE90"></a>

A typed markdown language for structured content.

Typedown extends from markdown:
1. A typedown document is a structured markdown document: It has YAML frontmatter with a specified schema to describe its properties and its relationship with other documents.
2. At its core, Typedown is a syntax and semantic analyzer for knowledge base. Based on this, several QOL features for editing notes are implemented:
   - Syntax highlighting.
   - Autocompletion.
   - Go to references.
   - Go to definition.
   - Semantic renaming.
3. Extended furthermore, static site generators, search engines can be built on top of the core compiler.

Here's a demo of a web generated from typedown:

<img width="2880" height="1582" alt="image" src="https://github.com/user-attachments/assets/2223934e-86ce-4e3c-bed3-2c2c5133c2fd" />

## Installation

See the [getting started guide](https://huydo862003.github.io/typerighter/guide/01-getting-started) for full installation and usage instructions.

```sh
npm install typerighter
```

> The npm package name `typedown` is unavailable (npm considers it too similar to `typedoc`). Since "down" was taken, we went the other direction: The entry package is [`typerighter`](https://www.npmjs.com/package/typerighter) and scoped packages are under [`@typerighter`](https://www.npmjs.com/org/typerighter)

## Design Documentation

The compiler design is researched and documented in the [dboxide](https://github.com/Huy-DNA/dboxide) repo. See the [design docs](https://github.com/Huy-DNA/dboxide/tree/main/doc/src/design) for details on the syntax, type system, and incremental compilation engine.

The tree-sitter grammar research is documented in the [loupe](https://github.com/huydo862003/loupe) repo.

NOTE: In this early phase, we don't intend to be conformant to [commonmark](https://commonmark.org/) though. However, basic markdowns with basic syntaxes like tables, lists, links, headings, bold texts, block quotes should just work. In the future, we can use these two to check for compatibility: [spec](https://spec.commonmark.org/) and [reference implementations & tests](https://github.com/commonmark).

For linting and formatting, we follow [Google's markdown style guide](https://google.github.io/styleguide/docguide/style.html) with some divergences. See [crates/typedown-lang/README.md](crates/typedown-lang/README.md) for details.

## Dev Setup

See [DEVELOPMENT.md](DEVELOPMENT.md) for full setup instructions (Nix and non-Nix).

## Editor Integration

| Editor            | User Guide                         | Development                                  |
| ----------------- | ---------------------------------- | -------------------------------------------- |
| Neovim            | [README](editors/nvim/README.md)   | [DEVELOPMENT](editors/nvim/DEVELOPMENT.md)   |
| Zed               | [README](editors/zed/README.md)    | [DEVELOPMENT](editors/zed/DEVELOPMENT.md)    |
| VSCode / VSCodium | [README](editors/vscode/README.md) | [DEVELOPMENT](editors/vscode/DEVELOPMENT.md) |

## Dependency Graph

- `typedown-macros` and `typedown-types` contain common utils, which are the lowest common denominator that everyone depends upon.
  - They can be depended upon by other crates.
  - They must not depend on any other crates.
- `typedown-incremental` contains the incremental engine.
  - It must not depend on any other crates, except for `typedown-macros` and `typedown-types`.
  - It can be depended upon by everyone, EXCEPT FOR `typedown-macros` and `typedown-types`.
- `typedown-lang` contains the AST structure, parser, typechecking, and evaluation logic for typedown.
  - It depends on `typedown-incremental`, `typedown-macros`, and `typedown-types`.
  - It must not depend on `typedown-server`/`typedown-build`.
  - It can only be depended upon by `typedown-server`/`typedown-build`.
- `typedown-server` contains the LSP server for typedown while `typedown-build` contains the build server.
  - It can depend on any other crates.
  - It can not be depended upon by others.

## Common Pitfalls (and Painful Lessons)

These are some lessons learnt during the development of the project. Some comments in the code are also marked with `TIL`.

### Visitor Pattern for Serialization/Hashing

There are two naive approaches to serialization, and a third that combines the best of both. This applies to why the built-in Hash trait chooses this design.

> I think this is related to the expression problem.

**Approach 1: Serializer knows every type**:

- There's a single serializer.
- The serializer has a method per type to serialize objects of that type.
- Adding a new type means modifying the serializer.

```rust
struct Serializer { buf: Vec<u8> }

impl Serializer {
    fn serialize_person(&mut self, p: &Person) {
        self.buf.extend(p.name.as_bytes());
        self.buf.extend(&p.age.to_le_bytes());
    }
    fn serialize_product(&mut self, p: &Product) { /* ... */ }
    // Every new type = new method here
}
```

**Approach 2: Each type serializes itself**:

- Each type handles its own serialization.
- Now every type must know the byte format, and changing the format means updating every type.

```rust
trait Serialize {
    fn serialize(&self, buf: &mut Vec<u8>);
}

impl Serialize for Person {
    fn serialize(&self, buf: &mut Vec<u8>) {
        buf.extend(self.name.as_bytes());  // must know the wire format
        buf.extend(&self.age.to_le_bytes());
    }
}
```

**Approach 3: Visitor (double dispatch)**. Split the responsibilities.

- The type decides WHAT to write (which fields, in what order).
- The serializer decides HOW to write it (byte format, endianness, buffering).
- Therefore, neither depends on the other's internals.

```rust
trait Serializer {
    fn emit_str(&mut self, v: &str);
    fn emit_u32(&mut self, v: u32);
}

trait Serialize {
    fn serialize(&self, s: &mut impl Serializer);
}

impl Serialize for Person {
    fn serialize(&self, s: &mut impl Serializer) {
        s.emit_str(&self.name);  // WHAT: name field
        s.emit_u32(self.age);    // WHAT: age field
    }
}
```

Adding a new type does not touch the serializer. Changing the byte format does not touch any type. This is how `std::hash` works (`Hash`/`Hasher`), how rustc does it (`Encodable`/`Encoder`), and how serde does it (`Serialize`/`Serializer`).

Reference: [rustc_serialize/src/serialize.rs](https://github.com/rust-lang/rust/blob/2371d697abddba53be85137d5a68064066b4ae10/compiler/rustc_serialize/src/serialize.rs)

### Vue SSR Pre-rendering

The general model ([Vue SSR guide](https://vuejs.org/guide/scaling-up/ssr)) has 2 main concepts:
1. The SSR bundle.
2. The client bundle.

### HMR in Vite

Two problems:
- **Race condition**: Vite's watcher fires before Rust finishes re-indexing, so we suppress `handleHotUpdate` for `.td` files and let the Rust RPC events (`onContentChanged`, etc.) drive invalidation instead.
- **Client-side hot reload**: `.td` files become Vue SFCs, so Vue's HMR runtime handles `import.meta.hot.accept()` automatically. No full page reload needed.

#### Making data hot-reloadable

The general idea: every piece of dynamic data lives in its own virtual module. The app entry imports each one and accepts HMR updates. When data changes on the server side, we invalidate the virtual module and push an HMR update. The client swaps in the new data without a full page reload.

On the client side, the app entry wires them up:

```js
import { pages as initialPages } from '@typedown/pages';
import initialSiteData from '@typedown/site-data';
import searchIndex from '@typedown/search-index';

let pages = initialPages;
// siteData and searchIndex are shallowRefs, set up in createTypedownApp

if (import.meta.hot) {
  import.meta.hot.accept('@typedown/pages', (m) => { pages = m.pages; });
  import.meta.hot.accept('@typedown/site-data', (m) => { siteData.value = m.default; });
  import.meta.hot.accept('@typedown/search-index', (m) => { searchIndexRef.value = m.default; });
}
```

Things to avoid:

1. **Don't inline data as JSON in the app entry**: It becomes part of the module. The only way to update it is to invalidate the entire app module, which forces a full page reload.
2. **Don't use `const` for data that needs HMR**: Closures capture the variable binding, not the value. With `let`, reassigning in the HMR handler means all closures that reference the variable see the new value on next call.
3. **Don't put `import.meta.glob` in the app entry**: The glob is evaluated when Vite transforms the module. Invalidating the app module to re-scan the glob forces a full reload. Put it in a separate virtual module instead.
4. **Don't `provide()` a plain object**: Vue cannot track mutations on a plain object passed through `provide`/`inject`. Use a `shallowRef` so that assigning `.value` triggers reactivity in all consumers.
5. **Don't use full reloads for data changes**: Reserve full reloads for structural changes that affect the app bootstrap itself (e.g. config changes that alter `basePath`).

### Parameterized Types vs Universal Types

These are different things despite both involving type parameters:

- **Parameterized type** (type constructor): `List :: Type -> Type`. Not a type by itself, needs args applied to become one (`List[string]`). This is what our `arity` system implements.
- **Universal type**: `forall T. T -> T :: Type`. Already a concrete type (kind `Type`). Values of this type are polymorphic functions (e.g. the identity function). The `forall` binds `T` internally.

A **higher-kinded type (HKT)** is a type _variable_ ranging over type constructors (e.g. `f` in `forall f. f Int -> f String` where `f` could be `List`, `Maybe`, etc.). We don't have this.

An **existential type** `exists T. SchemaProperty[T]` means "there is some `T`, but I don't tell you which". Useful for heterogeneous collections where each entry independently picks its `T`.

### Static Types vs Runtime Objects

We were conflating three things in a single `TdTypeLike` trait:

1. **Static type shape**: What the type checker sees (assignability, field types, generics)
2. **Runtime object shape**: What the evaluator sees (field access, method dispatch)
3. **User mental model of runtime shape**: What the user thinks values look like

Compare with real languages:

- **JavaScript**: at runtime, an object has a hidden class (V8's internal shape for optimization), a prototype chain (user-visible runtime shape), and no static types. The engine's internal shape and the user's mental model are different things.
- **TypeScript**: adds a third layer. The static type describes the value shape for the checker. At runtime, it is erased completely. The JS engine never sees it.
- **Rust**: at runtime, values are just bytes. No type objects, no reflection, no access to static information. The compiler knows everything, the binary knows nothing about types.

Our old design had the evaluator continuously querying the same type objects that the typechecker uses. The evaluator called `accepts()`, the typechecker called `construct()`. No enforcement of who accesses what. Schema evaluation and content evaluation both produced the same type representations.

The fix: split into two systems.

### Stable specialization via autoderef

We needed ad-hoc overloading: call one function for types implementing a trait, another function for everything else. Nightly `specialization` does this but is unsound and may never stabilize.

- Rust method resolution prefers the least-deref match.
- Implement the same method name on different traits at different reference depths.
- The compiler picks the one requiring fewer dereferences.
- Higher priority impls get more `&` layers on `Self`.

```rust
struct W<T>(T);

// Lower priority: impl on W<T> (no refs)
trait Default { fn check(&self) -> bool; }
impl<T> Default for W<T> { fn check(&self) -> bool { true } }

// Higher priority: impl on &W<T> (one ref)
trait Special { fn check(&self) -> bool; }
impl<T: Display> Special for &W<T> { fn check(&self) -> bool { false } }

// Compiler picks Special for Display types, Default for others
(&W(42i32)).check()  // false (Display)
(&W(Opaque)).check() // true  (no Display)
```

- Only works where the concrete type is known at the call site.
- Inside a generic function `fn foo<T>(val: T)`, the compiler cannot resolve which impl to pick because `T` is unknown.
- So this is only useful in macro-generated code.

Based on: https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html

### typeof via generic inference

Rust has no `typeof` operator. When generating code via macros, sometimes you have a value and need to create a type-level witness (like `PhantomData<T>`) without being able to name `T`.

- A generic helper function infers `T` from its argument.
- We use this in the `specialize!` macro to create a `PhantomData` wrapper that carries the type for autoderef dispatch.

```rust
fn phantom<T>(_: &T) -> Wrapper<T> { Wrapper(PhantomData) }

let val = some_expression();
let witness = phantom(&val);  // Wrapper<TypeOfVal> without naming the type
```

### LSP: Dynamic vs Static Registration

When a client advertises `dynamicRegistration: true` for `workspace.fileOperations` (as VSCode does), some clients **ignore** static capabilities declared in `InitializeResult`. The server must use `client/registerCapability` to dynamically register for `workspace/willRenameFiles` and `workspace/didRenameFiles` at runtime.
