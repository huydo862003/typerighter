---
_label: "Coding Conventions"
_icon: icon.document
---

Official site: [Link](https://rust-analyzer.github.io/book/contributing/style.html).

## General Philosophy

- `rust-analyzer`'s approach to clean code:
  - Velocity over perfection: Do not block functional PRs on purely stylistic changes.
  - "Show, don't just tell": For complex style issues, reviewers are encouraged to merge the PR and then send a follow-up cleanup PR themselves. This resolves the issue faster and teaches the author "by example" rather than through endless comment threads.
- If a review comment applies generally, update the Style Guide instead of leaving a one-off comment. This way, temporary feedback is turned into permanent documentation.
- Small, atomic cleanup PRs (even just renaming a variable) are explicitly encouraged to keep the codebase healthy.

## Scale of Changes

- Generally, small & focused PRs are preferred; but sometimes, that isn't possible.

- `rust-analyzer` categorizes PRs into 3 groups.

### Internal Changes (Low Risk)

- Definition: Changes confined to the internals of a single component. No `pub` items are changed or added (no changes to the interfaces and no new dependencies).
- Review standard: Easy Merge.
  - Does the happy path work?
  - Are there tests?
  - Does it avoid panicking on the unhappy path?

### API Expansion (Medium Risk)

- Definition: Adding new `pub` functions or types that expose internal capabilities to other crates.
- Review standard: High scrutiny.
  - The interface matters more than the implementation. It must be correct and future-proof.
- `rust-analyzer`'s guideline: If you start a "Type 1" change and realize you need to change the API, stop. Split the API change into its own separate, focused PR first.

### Dependency Changes (High Risk)

- Definition: Introducing new connections between components via pub use re-exports or `Cargo.toml` dependencies.
- Review standard: Rare & dangerous.
  - These break encapsulation.
  - Even an innocent-looking `pub use` can accidentally degrade the architecture by leaking abstractions across boundaries.

## Crates.io Dependencies

- Restrict external dependencies: Be extremely conservative with `crates.io` usage to minimize compile times and breakage risks.
- Do not use small "helper" libraries (allowed exceptions: `itertools`, `either`).
- Internalize utilities: Place general, reusable logic into the internal `stdx` crate rather than adding a dependency.
- Audit dependency tree: Periodically review `Cargo.lock` to prune irrational transitive dependencies.

### Rationale

- Compilation speed:
  - Rust compiles dependencies from source.
  - Avoiding bloat is the best way to keep build times and feedback loops fast.
- Transitive bloat:
  - Small "helper" crates often pull in deep chains of hidden dependencies (the "iceberg" effect).
- Stability & Security:
  - Reduce the risk of upstream abandonment, breaking changes, or supply chain attacks.
- Self-reliance: If logic is simple enough for a micro-crate, it belongs in the internal `stdx` library, not as an external liability.

## Commit Style

- Document for changelogs to avoid release burden on the maintainers.
- Changelogs > Clean history.

### Git History

- Clean git history is strongly encouraged but not mandated.
- Use a rebase workflow. It is explicitly acceptable to rewrite history (force push) during the PR review process.
- Before the final merge, use interactive rebase to squash small "fixup" commits into logical units.

### Commit Message & PR Description

- Do not `@mention` users in commit messages or PR descriptions.
  - Reason: Rebasing re-commits the message, spamming the mentioned user with duplicate notifications.
- User-centric titles: Write PR titles/descriptions describing the user benefit, not the implementation details.
  - Good: "Make goto definition work inside macros".
  - Bad: "Use original span for `FileId`".
- Changelog automation: You must categorize PRs so release notes can be auto-generated. Use one of two methods:
  - Title prefix: `feat:`, `fix:`, `internal:`, or `minor:` (e.g., `feat: Add hover support`).
  - Magic comment: `changelog [fix] Description here in the PR body`.
- Visuals: For UI changes, include a GIF in the description to demonstrate the feature.

## Linting

- Clippy is used.

## Code

### Minimal Tests

- Tests must use the absolute minimum code necessary to reproduce the case. Aggressively strip "noise" from copy-pasted real-world code.
- Format declarative code densely (e.g., `enum E { A, B }` on a single line) to keep the test concise, provided it remains readable.
- Unindented raw strings: Use `r#...#` literals for multiline fixtures. Ensure the content is unindented (starts at column `0`) so that character offsets in the test match the actual file positions exactly.
- Rationale:
  - Reduce visual noise and scrolling, making the actual test case immediately obvious.
  - Lower execution time and keeps debug logs clean.
  - Unindented formatting allows you to use your editor's "selection character count" to verify byte offsets directly, without needing to manually subtract indentation whitespace.

### Marked Tests

- Marked test: A technique used to verify that a specific, often hard-to-reach line of code was actually executed during a test.
- Use `cov_mark::hit!` (in code) and `cov_mark::check!` (in tests) to create a strictly unique link between a specific edge case in the implementation and its corresponding test.
- Principle: Only maintain one mark per test and one mark per code branch.
- Never place multiple marks in a single test, and never reuse the same mark across different tests.
- Rationale: This ensures that searching for a mark immediately reveals the single canonical test responsible for verifying that specific code branch, eliminating ambiguity.

### `#[should_panic]`

- `#[should_panic]` is prohibited - `None` and `Err` should be explicitly checked.
- Rationale:
  - `#[should_panic]` is a tool for library authors to make sure that the API does not fail silently when misused.
  - `rust-analyzer` is a long-running server, not a library. It must handle all input gracefully, even invalid input (returning `Err` or `None`). It should never intentionally crash.
  - Expected panics still dump stack traces into the test logs. This "noise" creates confusion, making it difficult to distinguish between a test verifying a panic and an actual bug causing a crash.

### `#[ignore]`

- Never ignore tests. Explicitly assert the wrong behavor and add a `FIXME` comment.
- Rationale:
  - Visibility: It ensures the test fails immediately if the bug is accidentally fixed (alerting you to update the test).
  - Safety: It proves the bug causes incorrect output rather than a server crash (panic), which is a critical distinction for a long-running service.

### Function Preconditions

#### Type Encoding

- Function's assumptions should be expressed in types.
- The caller must be enforced to provide them.

```rust
// GOOD
fn is_zero(n: i32) -> bool {
  ...
}

// BAD
fn is_zero(n: Option<i32>) -> bool {
   let n = match n {
       Some(it) => ...,
       None => ...,
   };
}
```

- Rationale:
  - The caller has more context as to why to the callee's assumptions do not hold.
  - The control flow is therefore more explicit at the call site.

#### Parse, Don't Validate

- Bad practice:
  - One function validates that the data is valid (validate the assumption).
  - Another function uses that data based on the assumptions.
- Good practice: Validate and immediately use the data in the same place (like `match` instead of bare `if`).

- Reasons:
  - The bad practice is prone to decay over time. The maintainer has to memorize the assumptions and make sure refactoring efforts of checks actually verify the assumptions.
  - The good practice always ensure that the assumptions hold when manipulating the data.

- Example from `rust-analyzer`

  ```rust
  // GOOD
  fn main() {
      let s: &str = ...;
      if let Some(contents) = string_literal_contents(s) {

      }
  }

  fn string_literal_contents(s: &str) -> Option<&str> {
      if s.starts_with('"') && s.ends_with('"') {
          Some(&s[1..s.len() - 1])
      } else {
          None
      }
  }

  // BAD
  fn main() {
      let s: &str = ...;
      if is_string_literal(s) {
          let contents = &s[1..s.len() - 1];
      }
  }

  fn is_string_literal(s: &str) -> bool {
      s.starts_with('"') && s.ends_with('"')
  }
  ```

- Remarks:
  - This pattern perfectly illustrates Robert Harper's concept of "Boolean Blindness". By reducing a complex check to a simple bool (true/false), we discard the proof of validity. The compiler sees a "true" flag, but it doesn't see the "valid data," forcing us to rely on faith later in the code.
  - I learned this the hard way while building a DBML parser. I designed a system where the "validation phase" was separate from the "execution phase". Because the validation step didn't return a new, safe type (it just returned true), the execution phase had to blindly trust that the validation had run correctly.
  - While systems like TypeScript uses Flow Typing to mitigate this (by inferring types inside `if` blocks), that safety is often local only. As soon as you pass that variable into a different function, the "flow context" is lost unless explicitly redefined.

### Control Flow

- Push "if"s up and "for"s down.

### Assertions

- Use `stdx::never!` liberally instead of `assert!`.
- `never!` checks a condition and logs a backtrace if it fails, but returns a `bool` instead of crashing. This allows you to write: `if stdx::never!(condition) { return; }`.
- Rationale: `rust-analyzer` is a long-running server. A bug in a minor feature (like a specific completion) should **log an error** and bail out of that specific request, not **crash** the entire IDE session.

### Getters & Setters

- Two cases to consider:
  - No invariants: If a field can hold any value safely, just make it `pub`. Don't write boilerplate code.
  - Invariants exist: If the data has rules (e.g., "cannot be empty"), make the field private, enforce the rule in the Constructor, and provide a Getter.
- Never provide setters. If data needs to change, it should likely be done via a specific behavior method or by creating a new instance, ensuring invariants are never bypassed.

- Getters should return borrowed data. `rust-analyzer`'s example:

  ```rust
  struct Person {
    // Invariant: never empty
    first_name: String,
    middle_name: Option<String>
  }

  // GOOD
  impl Person {
      fn first_name(&self) -> &str { self.first_name.as_str() }
      fn middle_name(&self) -> Option<&str> { self.middle_name.as_ref() }
  }

  // BAD
  impl Person {
      fn first_name(&self) -> String { self.first_name.clone() }
      fn middle_name(&self) -> &Option<String> { &self.middle_name }
  }
  ```

- Rationale:
  - The APIs are internal so (internal) breaking changes can be allowed to move fast:
    - Using a `pub` field (with no invariants) introduces less boilerplate but may be breaking if the `pub` field is suddenly imposed an invariant and has to be changed to private.
    - Using an accessor can prevent breaking changes, but it means implicitly promising a contract and imposing some maintenance boilerplate.
  - Privacy helps make invariants local to prevent code rot.
  - A type that is too specific (borrow owned types like `&String`) leaks irrelevant details (neither right nor wrong), which creates noise and the client may accidentally rely on those irrelevent details.

### Useless Types

- Prefer general types.
- If generality is not important, consistency is important.

```rust
// GOOD      BAD
&[T]         &Vec<T>
&str         &String
Option<&T>   &Option<T>
&Path        &PathBuf
```

- Rationale:
  - General types are more flexible.
  - General types leak fewer irrelevant details (which the client may accidentally rely on).

### Constructors

- If a `new` function accepts zero arguments, then use the `Default` trait (either derive or manually implemented).
  - Rationale:
    - Less boilerplate.
    - Consistent: Less cognitive load for the caller - "Should I call `new()` or `default()`?"
- Use `Vec::new` instead of `vec![]`.
  - Rationale:
    - Strength reduction.
    - Uniformity.
- Do not provide `Default` if the type doesn't have sensible default value (many possible defaults or defaults that has invalid states).
  - Preserve invariants.
  - The user does not need to wonder if the provided default is their desired initial values.

```rust
// GOOD
#[derive(Default)] // 1. Best case: Derive it automatically
struct Options {
    check_on_save: bool,
}

// GOOD (Manual Implementation)
struct Buffer {
    data: Vec<u8>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            // 2. Use Vec::new() instead of vec![] (Strength Reduction)
            // It is semantically lighter (function vs macro) and more uniform.
            data: Vec::new(),
        }
    }
}

// BAD
struct OptionsBad {
    check_on_save: bool,
}

impl OptionsBad {
    // 3. Avoid zero-arg new().
    // It forces users to remember "Do I call new() or default() for this type?"
    fn new() -> Self {
        Self { check_on_save: false }
    }
}
```

### Functions Over Objects

- Public API: Prefer simple functions (`do_thing()`) over transient objects that exist only to execute one method (`ThingDoer::new().do()`).
- Internal logic: It is acceptable (and encouraged) to use "Context" structs _inside_ the function to manage complex state or arguments during execution.
- Rationale:
  - The "Iceberg" pattern: The user sees a simple function interface; the developer uses a structured object implementation behind the scenes.
  - Implementor API is not mixed with user API.
- Middle ground: If a struct is preferred for namespacing, provide a static `do()` helper method that handles the instantiation and execution in one step.
- Rationale:
  - Reduce boilerplate for the caller.
  - Prevent implementation details (like temporary state management) from leaking into the public API.

```rust
// BAD (Caller has to build and run)
ThingDoer::new(arg1, arg2).do();

// GOOD (Caller just acts)
do_thing(arg1, arg2);

// ACCEPTABLE INTERNAL IMPLEMENTATION (Using a struct to organize code)
pub fn do_thing(arg1: Arg1, arg2: Arg2) -> Res {
    // The struct is an implementation detail, hidden from the user
    let mut ctx = Ctx { arg1, arg2 };
    ctx.run()
}
```

### Functions With Many Parameters

- Use `Config` struct:

  ```rust
  // BAD (Call site is confusing)
  fn annotations(db, file_id, true, false, true);

  // GOOD (Call site is explicit and flexible)
  fn annotations(db, file_id, AnnotationConfig {
      binary_target: true,
      annotate_runnables: false,
      annotate_impls: true,
  });
  ```

- Rationale:
  - Call site is clearer.
  - Encapsulating volatile parameters in a struct shields intermediate functions from breaking changes, allowing you to add new options without updating the signature of every function in the call chain.

- No `Default` for `Config`: Do not implement `Default`. Force the caller to provide explicit context.
  - Rationale: They know better than the struct what the initial state should be.

- Pass configuration as an argument to the function, do not store it in the object's state.
  - Rationale: This allows the same object to handle multiple requests with different configurations dynamically.

- `Command` pattern: If a set of parameters can yield different return types (e.g., `Vec<T>` vs `Option<T>`), wrap parameters in a "Command" struct.
  - Rationale: This avoids creating multiple top-level functions (`query_all`, `query_first`) with identical argument lists.

```rust
// GOOD (Command Pattern)
// Captures arguments once, offers multiple execution paths
pub struct Query {
    pub name: String,
    pub case_sensitive: bool,
}

impl Query {
    // Return type A
    pub fn all(self) -> Vec<Item> { ... }
    // Return type B
    pub fn first(self) -> Option<Item> { ... }
}

// BAD (Parameter Duplication)
// Requires repeating arguments for every variation of the result
fn query_all(name: String, case_sensitive: bool) -> Vec<Item> { ... }
fn query_first(name: String, case_sensitive: bool) -> Option<Item> { ... }
```

- Remarks: Does the `Command` rule conflict with the rule "Do not store `Config` in state"?
  - The `Config` rule applies to long-lived services (e.g., `Database`). These should remain stateless so they can handle diverse requests without needing to be reset.
  - The `Command` rule applies to short-lived tasks (e.g., `Query`). These objects exist solely to bundle parameters for a single operation and are discarded immediately after use.
  - Relationship: The "Command" is effectively a temporary container that you pass to (or use with) the "Service".

### Prefer Separate Functions Over Parameters

- Split "flag" arguments: If a function is solely invoked with hardcoded literals (`true`/`None`), refactor it into distinct named functions (e.g., `process_fast()` vs `process_full()`) to eliminate internal branching and "false sharing" of unrelated logic.

- Rationale:
  - Functions with flag arguments often display false sharing. There's often `if` branching to distinguish between different cases.
    ```rust
    // Caller
    process_data(true);
    process_data(false);

    // Callee
    fn process_data(is_fast: bool) {
        if is_fast {
            // Fast algorithm
        } else {
            // Accurate algorithm
        }
    }
    ```
  - These functions seem to share logic and are related and it seems that it makes sense to merge them into 1 function. However, over time, they diverge. Therefore, splitting the control flows into separate functions simplify the logic & eliminate irrelevant details (such as the maintainer would fail to see the cross-dependencies between the `if` branches).
  - Split the common code of the `if` branches into a common helper instead.

- Remark: This [binder-refactoring PR](https://github.com/holistics/dbml/pull/530) is a hard lesson for me illustrating this point, both the problem and the solution (splitting into separate classes & extract common helpers). Although there is still lots of room for improvement, this is already significantly better.

### Appropriate String Types

- When calling OS APIs (file systems, env vars, arguments), use `OsString` and `&OsStr`, never `String` or `&str`.
- Rust `String` guarantees valid UTF-8. Operating Systems do not.
  - Linux/Unix: Paths are arbitrary byte sequences (except `null`).
  - Windows: Paths are potentially ill-formed UTF-16 sequences.
- Rationale: This creates a strict type-level boundary.
  - If you hold a `String`, you know it is safe, clean text.
  - If you hold an `OsString`, you know it is "dirty" data from the outside world.
  - Using `OsString` prevents accidental panics or data corruption when encountering a file named with invalid encoding.

- Avoid the standard `std::Path`, use the custom `AbsPathBuf` and `AbsPath` wrapper that guarantees the path inside is absolute.
- Rationale:
  - CWD is global mutable state.
  - If you use a relative path like `Path::new("src/main.rs")`, the OS resolves it relative to where the *server binary* started, not where the *project* is.

## Premature Pessimization

### Avoid Allocations

- Zero-allocation default: Prefer stack-based structures (iterators) over heap-based collections (`Vec`, `String`) unless you specifically need ownership or long-term storage.
- Lazy vs eager: `collect::<Vec>()` eagerly processes the entire sequence and allocates memory immediately. Iterators are lazy and compute items only on demand.

```rust
use itertools::Itertools;

// BAD (Heavy)
// 1. Allocates heap memory. 2. Processes entire string. 3. Frees memory.
let parts: Vec<&str> = text.split(',').collect();
if parts.len() == 3 {
    process(parts[0], parts[1], parts[2]);
}

// GOOD (Light)
// 1. No allocation. 2. Stops after 3 items. 3. Stores ptrs on Stack.
if let Some((a, b, c)) = text.split(',').collect_tuple() {
    process(a, b, c);
}
```

### Push Allocations to the Call Site

- Rationale: The cost of calling a function becomes more explicit.

### Collection Types

- `rustc_hash`'s map and set are preferred over those in `std::collections`.

- Rationale:
  - Faster hasher.
  - Slightly reduce code size if applied consistently.

### Avoid Intermediate Collections

- Use an accumulator parameter (list, set, map) as the first parameter to collect values.

### Avoid Monomorphization

- Minimize generics: Avoid heavy generic logic at crate boundaries to prevent compile-time bloat caused by monomorphization (code duplication).
- "Inner dyn" pattern: Use thin generic wrappers that immediately delegate to private, non-generic functions using dynamic dispatch (`dyn Trait`).
- Concrete types: Avoid `AsRef` polymorphism; prefer concrete types like `&Path` unless writing a widely-used library.
- Rationale: Optimize for compile speed by default; runtime performance only matters for the "hot" 20% of code, but compilation costs affect 100%.

```rust
use std::fmt::Display;

// --- 1. The Wrapper (Generic) ---
// This function is "monomorphized" (duplicated) for every type T.
// But since it's only 1 line, the duplication cost is negligible.
pub fn log_message<T: Display>(msg: T) {
    // Coerce the specific type T into a trait object (&dyn Display)
    log_message_impl(&msg);
}

// --- 2. The Implementation (Dynamic) ---
// This function is compiled ONLY ONCE.
// It handles the heavy lifting via dynamic dispatch (vtable).
fn log_message_impl(msg: &dyn Display) {
    // Imagine 500 lines of complex logging logic here...
    println!("Timestamp: [INFO] {}", msg);
    // ... extensive I/O, formatting, or network calls ...
}

fn main() {
    log_message("Hello");       // T is &str -> Wrapper created for &str
    log_message(42);            // T is i32  -> Wrapper created for i32
    log_message(3.14);          // T is f64  -> Wrapper created for f64

    // Result: 3 tiny wrappers, but the heavy `log_message_impl` exists only once.
}
```

## Style

### Order of Imports

- Modules first: Declare modules (`mod x;`) at the top, before any imports, ordered by "suggested reading."
- Import structure:
  - Grouping: Use one `use` statement per crate (group items inside `{ ... }`).
  - Spacing: Separate different groups with blank lines.
  - Import order:
    1. `std`: Standard library (`use std::...`).
    2. External: Third-party and workspace crates.
    3. Local: Current crate (use `crate::...`).
    4. Relative: Parent/child modules (use `super::...`), though `crate::` is preferred.
- Re-exports: Place `pub use` after all imports, as they are treated as item definitions.
- Rationale: Ensures consistency, improves readability for new contributors, and highlights dependencies clearly.

### Import Style

- Qualify layer types: Always qualify items from `hir` and `ast` to prevent ambiguity and clarify the architectural layer.
  - Good: `use syntax::ast; ... func: hir::Function`
  - Bad: `use hir::Function; ... func: Function`
- Trait implementations: Import the module (e.g., `std::fmt`), not the trait itself, when implementing standard traits.
  - Good: `impl fmt::Display for ...`
- Rationale: Reduces typing and clearly distinguishes implementation from usage.
- Avoid local globs: Do not use use `MyEnum::*;` inside functions.
- Absolute paths: Prefer `use crate::foo` over relative paths like `super::` or `self::` for consistency.
- No re-exports: Avoid re-exports in non-library code to prevent multiple access paths and maintain consistency.

### Order of Items

- Public API first: Always place public items (`pub` or `pub(crate)`) at the very top, before any private helpers or implementation details.
- Types before logic: Define data structures (`struct`, `enum`) before functions and `impl` blocks.
- Top-down: Order type definitions by dependency, place the "parent" container before the "child" component it contains.
- Rationale:
  - Optimize for a new reader scanning top-to-bottom.
  - When code is folded, the file structure should read like API documentation.

### Context Parameters

- Context-first: Always pass "context" parameters (invariant data threaded through many calls) as the **first** arguments.
  - Rationale:
    - This creates a visual hierarchy: "setting"  "actors".
    - It mimics the `self` convention in OOP (where the context/object always comes first).
    - When scanning a function call, you immediately see *where* the operation is happening (the context) before seeing *what* is being processed (the variable data).

```rust
// BAD: Context (db) is hidden at the end.
// In a long list of arguments, you might miss which 'db' is being used.
fn transform_data(data: String, id: usize, force: bool, db: &Database) { ... }

// Usage
transform_data(raw_input, 42, true, &primary_db);

// GOOD: Context (db) is front and center.
// It establishes the environment immediately.
fn transform_data(db: &Database, data: String, id: usize, force: bool) { ... }

// Usage
transform_data(&primary_db, raw_input, 42, true);

```

- If there are multiple context parameters, bundle them into a struct and pass it as `&self`.

```rust
// BAD: Parameter explosion.
// Every helper function needs to accept all three context items.
fn parse(db: &Db, cfg: &Config, cache: &Cache, text: &str) {
    validate(db, cfg, cache, text); // Tedious repetition
}

// GOOD: Packed Context.
struct ParseCtx<'a> {
    db: &'a Db,
    cfg: &'a Config,
    cache: &'a Cache,
}

impl<'a> ParseCtx<'a> {
    // The signature is clean. Context is implicit in `&self`.
    fn parse(&self, text: &str) {
        self.validate(text);
    }

    fn validate(&self, text: &str) { ... }
}

```

- The "dangling argument" problem:
  - Context-first works better when non-context parameters are lambdas (closures).
  - Rationale:
    - Rust closures often span multiple lines.
    - If the context parameter is placed *after* the closure, it ends up "dangling" after the closing brace `}`. This is visually confusing and looks like a syntax error or a forgotten fragment.
    - Placing context *first* ensures the closure body is the last thing the eye sees, resulting in a clean termination of the statement.

```rust
// BAD: Context Last
// The `&context` argument is easy to miss or looks like it belongs
// to a different statement because it follows a large code block.
apply_changes(|item| {
    // ... complex multi-line logic ...
    // ... complex multi-line logic ...
    item.finalize()
}, &context); // <--- The "Dangler"


// GOOD: Context First
// The statement starts with the context, and the closure flows naturally
// until the end of the function call.
apply_changes(&context, |item| {
    // ... complex multi-line logic ...
    // ... complex multi-line logic ...
    item.finalize()
}); // <--- Clean, standard closure syntax
```

### Variable Naming

- Prioritize verbose & boring but clear names: prefer long names that mirror the type (e.g., `global_state: GlobalState`). Rely on code completion, not brevity.
- Standard variables:
  - `res`: Function result.
  - `it`: Generic item (when identity is irrelevant).
  - `n_foos`: Count (preferred over `foo_count`).
  - `foo_idx`: Index.
- Keyword collisions: Avoid `r#ident` syntax. Use these consistent replacements: `crate` -> `krate`, `enum` -> `enum_`, `fn` -> `func`, `impl` -> `imp`, `macro` -> `mac`, `mod` -> `module`, `struct` -> `strukt`, `trait` -> `trait_`, `type` -> `ty`.
- Spelling & acronyms:
  - Use American spelling (`color`).
  - Avoid ad-hoc acronyms; stick to common ones (`db`, `ctx`).

### Error Handling Trivia

- Use `anyhow::Result` instead of the bare `Result`.
  - Rationale: Makes the return type immediately clear without checking imports.
- Macro choice: Prefer `anyhow::format_err!` over `anyhow::anyhow`.
  - Rationale: More "boring" (standard), consistent, and avoids the stuttering of `anyhow::anyhow`.
- Message formatting:
  - There are no strict rules on message structure. Rust standard library uses lowecase, while `anyhow` uses uppercase.
  - Do not end error or context messages with a period (`.`).

### Early Returns

- Handle negative cases immediately with an early return rather than wrapping the "happy path" inside a large if/else block.
- Rationale: Flattens code nesting and reduces "cognitive stack usage" (mental load).
- Explicit error returns: Use `return Err(e)` to exit with an error. Avoid using `Err(e)?` to simulate a throw.
- Rationale: `return` evaluates to the "never type" (`!`), which allows the compiler to strictly identify dead code, whereas `?` resolves to a generic type that can mask unreachable code.

### Comparisons

- Prefer less-than:
  - Always use `<` or `<=` comparisons.
  - Avoid `>` or `>=`.
- Rationale:
  - Spatial intuition: Corresponds to the real number line where values increase from left to right (`0->infinity`).
  - Visual ordering: `lo <= x && x <= hi` visually places `x` in the middle, whereas `x >= lo` forces a mental "flip."

### If-let

- Prefer `match` over `if let ... else`: When you need to handle both the "success" and "failure" cases, use a full match statement.
- Rationale:
  - Compactness: `match` is usually cleaner and requires less syntax for simple alternatives.
  - `Precision`: The else block in `if let` is implicit (it covers everything else). `match` forces you, or allows you, to be explicit about what the negative case is (e.g., `None` vs `Err(_)`), making the code more robust to type changes.

### Match Ergonomics

- Avoid `ref`: Do not use the `ref` keyword in patterns.

Rationale:
  - Obsolescence: `ref` is largely redundant due to "match ergonomics" (introduced in recent Rust editions).
  - Simplicity: Relying on `match` ergonomics is cleaner and avoids mixing legacy syntax with modern style.

### Empty Match Arms

- Use the unit value `()`, for empty match arms, rather than an empty block `{}`.

```rust

// GOOD
Err(err) => error!("{}", err),
Ok(_) => (), // <--- Clean, single-line comma style

// BAD
Err(err) => error!("{}", err),
Ok(_) => {}  // <--- Block style breaks the visual rhythm
```

- Rationale:
  - In Rust, `()` is the value "nothing," while `{}` is a block of code that evaluates to nothing.
  - While they are functionally identical here, using `=> ()`, keeps the match arm visibly distinct as a "value" rather than a "scope," maintaining a consistent visual rhythm in long match statements.

### Functional Combinators

- Use functional combinators (`map`, `and_then`) only when they fit naturally.
- Prefer imperative control flow (`if`, `for`, `match`) over "forced" combinators like `bool::then` or `Option::filter`.
- The philosophy: Code should be dense in computation (doing work) but sparse in structure (fewer indirections per line).
- Rationale:
  - Rust has strong support for imperative flow (loops, early returns).
  - Rust functions are "less first-class" because of effects like `?` (try-operator) and `.await`. These do not compose well inside long chains of closures (e.g., trying to `?` inside a `filter` closure is painful).

### Turbofish

- Prefer explicit type ascription (`let x: Vec<i32> = ...`) over the "turbofish" syntax (`...collect::<Vec<i32>>()`).
- Rationale: We want to able to read everything from left-to-right without maintaining to much context:
  - Good: `let names: Vec<String> = users.iter().map(...).collect();`
    - Why: When you read the line left-to-right, you immediately know what is being built (`Vec<String>`). This context helps you understand the complex iterator chain that follows.
  - Bad: `let names = users.iter().map(...).collect::<Vec<String>>();`
    - Why: You have to read the entire chain until the very end to figure out what type is actually being produced.
  - No placeholders (`_`): Avoid `let x: Vec<_> = ...`.
- Rationale: If the compiler struggles to infer the type (forcing you to add a hint), a human reader will likely struggle too. Be kind to the reader and write the full type `Vec<i32>`.

### Helper Functions

- Avoid single-Use helpers: Do not create a separate function if it is only called once.
- Alternative: Use a block `{ ... }`.
- Rationale: This isolates the scope but keeps access to the context variables.
- Exception: Create a function if you need early returns (`return`) or error propagation (`?`).

- Local helpers: Place nested helper functions at the end of the enclosing function.
  - Structure: Main logic -> `return result;` -> `fn helper() { ... }`.
  - Limit: Do not nest more than 1 level deep.

### Helper Variables

- Create boolean helper variables for complex conditions (e.g., inside match guards).
- Rationale:
  - Act as a "cognitively cheap" abstraction (names the logic without hiding the context).
  - Make debugging easier (you can inspect/print the variable).

### Syntax Macros

- Tokens: Use the `T![token]` macro instead of `SyntaxKind::TOKEN_KW`.
- Example: `T![true]` instead of `SyntaxKind::TRUE_KW`.
- Rationale: Familiar syntax, avoids ambiguity (e.g., curly brace vs bracket).

### Documentation

- Comments: Write proper sentences. Start with a capital, end with a dot `.`.
- Markdown: Use sentence-per-line. Do not wrap lines hard; press Enter after every sentence.
- Rationale:
  - Makes diffs cleaner and editing easier.
  - Formatting a comment as a sentence (capital + period) forces the brain to switch from "scribbling notes" to "explaining thoughts."
  - Context dump: It tricks you into emptying your "mental RAM" (assumptions, constraints) into the code, rather than leaving vague fragments like `// fix this`.
