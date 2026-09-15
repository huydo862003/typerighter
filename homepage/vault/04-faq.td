---
_label: "FAQ"
_icon: icon.help
---

Answers to common questions that don't fit neatly into the guide or reference.

## Can I use Typedown without schemas?

Yes. A `.td` file with no `_type` field is treated as an untyped document. You can write plain frontmatter and markdown without defining any schema. Schemas are opt-in: add them when you want autocompletion, type checking, and validated relations.

## Can I use my existing markdown files?

Rename `.md` files to `.td` and they will work as body-only documents. Standard markdown syntax is supported. To add structured data, insert a YAML frontmatter block between `---` markers at the top of the file.

Typedown extends standard markdown with string interpolation (`${...}`), callout blocks (`:::`), math (`$` and `$$`), and container shorthands (`[[toc]]`). These are additive and do not break existing markdown.

## How is this different from Obsidian?

Obsidian uses `[[wiki links]]` to connect notes. Links are untyped: any file can link to any other file, and there is no schema to validate the connection. Obsidian is a desktop app with a rich plugin ecosystem.

Typedown uses `fref("path")` with schema-typed edges. The type checker ensures that a Task's `assignee` field actually points to a Person, not another Task. Typedown runs as an LSP server inside your existing editor (Neovim, VS Code, Zed) and generates a static site from the command line.

Both store content as plain text files in git. See the [comparison table](index#how-it-compares) on the homepage for a side-by-side overview.

## How is this different from Notion?

Notion stores data in a proprietary cloud format. Typedown stores data as plain `.td` files on disk, versioned in git. Notion has real-time collaboration and a polished web UI. Typedown has a type system, an LSP, and a static site generator.

If you need multi-user collaboration, Notion is the better tool. If you want typed structured content in plain files with editor-grade tooling, Typedown is a better fit.

## Does it work offline?

Yes. Everything runs locally. The LSP server, the dev server, and the build pipeline all work without an internet connection. Your content is plain files on disk.

## What happens when I rename a file?

Use your editor's rename command (F2 in VS Code, `<leader>rn` in Neovim). The LSP performs a semantic rename: it updates the file path and every `fref` that points to it across the vault. Manual renames also work, but you will need to update `fref` paths yourself.

## Can I use Typedown for a blog?

Yes. Define a schema like `Post` with fields for title, date, tags, and author. Each post is a `.td` file conforming to that schema. The built-in static site generator produces a searchable site with sidebar navigation, per-page icons, and prev/next links.

## What languages are supported in code blocks?

Code blocks use [Shiki](https://shiki.style/) for syntax highlighting, which supports all languages that VS Code supports. Specify the language after the opening triple backticks:

````markdown
```python
def greet(name):
    return f"Hello, {name}!"
```
````

## Can I nest schemas?

Yes, in two ways. Inline a mapping in the `type` field to define a nested object without a separate schema file. Or create a separate schema and reference it by name for reuse across multiple schemas. See [Schemas and types](02-guide/02-schemas-and-types#nested-objects) in the guide.

## How do I deploy the generated site?

Run `typerighter build` to produce a static site in `dist/`. Deploy it to any static file host: GitHub Pages, Netlify, Vercel, Cloudflare Pages, or a plain web server. No server-side runtime is needed.

## What markdown features are not supported?

Typedown supports most of CommonMark and several extensions, but some features from other tools are not available:

- **`[[wiki links]]`**: Use `fref("path/to/file.td")` in frontmatter and `[text](path)` in the body instead. Typedown uses explicit typed references rather than implicit wiki-style links.
- **Raw HTML tags**: Inline HTML (`<div>`, `<span>`, etc.) is not rendered. Use callout blocks (`:::`) and container shorthands (`[[component]]`) for custom layout.
- **Indentation-based code blocks**: Only fenced code blocks (triple backticks) are supported. Indenting text by 4 spaces does not create a code block.
- **Footnotes**: Not currently supported.
- **Embeds / transclusion**: Obsidian's `![[file]]` embed syntax is not supported. Use `fref` to link and traverse data across files instead.
- **Dataview queries**: Typedown does not have an inline query language yet. Computed collections are on the roadmap.
- **YAML arrays in tags**: Tags must be written as a YAML list (`- "tag"`), not as inline comma-separated values.
- **Front matter aliases**: Obsidian's `aliases` field has no equivalent. Use `_label` for the display name.

## Where do I report bugs or request features?

Open an issue on [GitHub](https://github.com/huydo862003/typerighter/issues). For questions and discussion, use [GitHub Discussions](https://github.com/huydo862003/typerighter/discussions).
