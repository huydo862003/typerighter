---
_label: "Deploying"
_icon: icon.upload
description: "Build a static site and deploy it to GitHub Pages, Netlify, Vercel, or any static host."
---

Typedown generates a fully static site with no server-side runtime. This page covers the build process and deployment to common hosting platforms.

## Build

Run `typerighter build` from your project root. The output goes to `dist/` by default.

```sh
typerighter build
```

The build runs the vault through the type checker, renders every page to HTML, and bundles the client-side JavaScript and CSS. The result is a self-contained directory you can serve from any static file host.

You can customize the output directory and the base path:

```sh
typerighter build --outDir public --base /docs/
```

## Preview locally

Before deploying, preview the production build to make sure everything looks right.

```sh
typerighter preview
```

This starts a local server that serves the `dist/` directory exactly as a static host would.

## GitHub Pages

Add a GitHub Actions workflow that builds the site and deploys it on every push to `main`.

```yaml
# .github/workflows/deploy.yml
name: Deploy
on:
  push:
    branches: [main]
jobs:
  deploy:
    runs-on: ubuntu-latest
    permissions:
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - run: npm install -g typerighter
      - run: typerighter build --base /your-repo-name/
      - uses: actions/upload-pages-artifact@v3
        with:
          path: dist
      - uses: actions/deploy-pages@v4
        id: deployment
```

Set `--base` to your repository name so asset paths resolve correctly under the GitHub Pages subdirectory.

## Netlify

Netlify detects static sites automatically. Add a `netlify.toml` at the project root:

```toml
[build]
  command = "npx typerighter build"
  publish = "dist"
```

Push to your git remote and Netlify builds and deploys on every commit.

## Vercel

Add a `vercel.json` at the project root:

```json
{
  "buildCommand": "npx typerighter build",
  "outputDirectory": "dist"
}
```

Or configure the build command and output directory in the Vercel dashboard.

## Cloudflare Pages

In the Cloudflare Pages dashboard, set:

- **Build command**: `npx typerighter build`
- **Build output directory**: `dist`

Cloudflare Pages deploys on every push to your connected git repository.

## Any static server

The `dist/` directory is a standard static site. Serve it with any web server:

```sh
# Python
python3 -m http.server -d dist 8080

# Node
npx serve dist

# Caddy
caddy file-server --root dist --listen :8080
```
