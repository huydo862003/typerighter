---
_label: "Rustc Development Guide"
_icon: icon.folder
---

[[directory-index]]

Basically the [rust-analyzer](../01-rust-analyzer/index) has covered most of AST design & query-based compilation. However, it (seems to) only cover incrementalism inside a process. What if I want incrementalism to survive across recompilations? Rustc as a compiler would really want this, so it covers the following:

- [Incremental Compilation in Detail](01-incremental-compilation)
- [Cache Persistence](02-cache-persistence)
- [StableHash Infrastructure](03-stable-hash)
- [Glossary](04-glossary)
