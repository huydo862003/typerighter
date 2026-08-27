Command-line interface for the `typerighter` package.

## `typerighter init`

Scaffold a new Typedown project. Creates a `typedown.yaml` config and a `vault/` directory with a starter `index.td`.

```sh
typerighter init [root]
```

- `root`: project directory (defaults to the current directory)

## `typerighter dev`

Start a development server with hot module replacement.

```sh
typerighter dev [root]
```

- `root`: project directory (defaults to the current directory)
- `--port <port>`: port to listen on (defaults to 8686)
- `--host`: expose the server to the network

## `typerighter build`

Build the site for production. Outputs static HTML, CSS, and JS to the output directory.

```sh
typerighter build [root]
```

- `root`: project directory (defaults to the current directory)
- `--outDir <dir>`: output directory (defaults to `dist`)
- `--base <path>`: base public path (defaults to `/`)

## `typerighter preview`

Preview the production build locally.

```sh
typerighter preview [root]
```

- `root`: project directory (defaults to the current directory)
- `--port <port>`: port to listen on

## `typerighter check`

Check the vault for type errors, unresolved references, and lint warnings. Exits with code 1 if there are errors.

```sh
typerighter check [root]
```

- `root`: project directory (defaults to the current directory)
- `--fix`: auto-fix formatting issues
