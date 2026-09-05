{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rust-stable = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
            "rustfmt"
          ];
          targets = [
            "wasm32-wasip1"
            "wasm32-wasip2"
            "wasm32-unknown-unknown"
          ];
        };
        # wasi-sdk does not exist :(
        # this is a standard nix derivation tho
        wasi-sdk = pkgs.stdenv.mkDerivation {
          name = "wasi-sdk-25.0";
          src = pkgs.fetchurl {
            url = "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-25/wasi-sdk-25.0-x86_64-linux.tar.gz";
            sha256 = "sha256-UmQN3hNZm/EnqVSZ5h1tZAJWEZRW0a+Il6tnJbzz2Jw=";
          };

          # the packages existent in the build shell
          nativeBuildInputs = [ pkgs.autoPatchelfHook ];
          buildInputs = [
            pkgs.stdenv.cc.cc.lib
            pkgs.zlib
            pkgs.ncurses
          ];

          sourceRoot = ".";

          installPhase = ''
            mkdir -p $out # out is nix build output dir or sth
            cp -r wasi-sdk-25.0-x86_64-linux/* $out/
          '';
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust-stable;
          rustc = rust-stable;
        };
        typedown-server = rustPlatform.buildRustPackage {
          pname = "typedown-server";
          version = "0.24.1";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "-p"
            "typedown-server"
          ];
          doCheck = false;
        };
      in
      {
        packages.typedown-lsp = typedown-server;
        packages.typedown-rpc = typedown-server;
        packages.default = typedown-server;

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            mdbook
            mdbook-mermaid
            rust-stable
            nodejs
            pnpm
            clang
            clang-tools
            zed-editor
            vscodium
            neovim
            cargo-edit
            cargo-watch
            wasm-pack
            tree-sitter
          ];
          TREE_SITTER_WASI_SDK_PATH = "${wasi-sdk}";
          shellHook = ''
            export TREE_SITTER_PATH="${pkgs.tree-sitter}/bin/tree-sitter"

            # Symlink our patched wasi-sdk so Zed's `install dev extension` works on NixOS
            mkdir -p "$HOME/.local/share/zed/extensions/build"
            ln -sfn "${wasi-sdk}" "$HOME/.local/share/zed/extensions/build/wasi-sdk"
          '';
        };
      }
    );
}
