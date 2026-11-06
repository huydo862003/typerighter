# typedown-server

![Binary Size](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/huydo862003/typerighter/badges/binary-size.json)

LSP server and JSON-RPC backend for Typedown. Communicates over TCP with editor extensions and the Vite plugin.

This crate produces the `typedown-lsp` binary, which provides:

- Language Server Protocol (LSP) for editor features
- JSON-RPC interface for the static site generator
- Incremental compilation via `typedown-incremental`
