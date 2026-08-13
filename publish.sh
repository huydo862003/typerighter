#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${IN_NIX_SHELL:-}" ]]; then
  echo "Error: run this script inside the Nix dev shell: nix develop"
  exit 1
fi

CURRENT_VERSION=$(cat VERSION)
echo "Current version: $CURRENT_VERSION"
echo ""

# Prompt bump type
echo "Select bump type:"
echo "  1) patch"
echo "  2) minor"
echo "  3) major"
echo "  4) prepatch"
echo "  5) preminor"
echo "  6) premajor"
echo "  7) prerelease"
read -rp "Choose [1-7]: " BUMP_CHOICE

case "$BUMP_CHOICE" in
  1) BUMP_TYPE="patch" ;;
  2) BUMP_TYPE="minor" ;;
  3) BUMP_TYPE="major" ;;
  4) BUMP_TYPE="prepatch" ;;
  5) BUMP_TYPE="preminor" ;;
  6) BUMP_TYPE="premajor" ;;
  7) BUMP_TYPE="prerelease" ;;
  *) echo "Invalid choice"; exit 1 ;;
esac

PREID=""
if [[ "$BUMP_TYPE" == pre* ]]; then
  read -rp "Preid (e.g. rc, alpha, beta) [rc]: " PREID
  PREID="${PREID:-rc}"
fi

# Compute new version
VERSION=$(pnpm dlx semver -i "$BUMP_TYPE" ${PREID:+--preid "$PREID"} "$CURRENT_VERSION")

# Staging releases use staging/v* tag, production use v*
if [[ "$BUMP_TYPE" == pre* ]]; then
  TAG="staging/v$VERSION"
else
  TAG="v$VERSION"
fi

echo ""
echo "New version: $VERSION  ($TAG)"
echo ""

# Open editor for changelog entry (production only)
if [[ "$BUMP_TYPE" != pre* ]]; then
  DATE=$(date +%Y-%m-%d)
  PLACEHOLDER="<!-- Enter changelog entry here, then save and close -->"

  {
    printf "## [%s] - %s\n\n%s\n\n" "$VERSION" "$DATE" "$PLACEHOLDER"
    [[ -f CHANGELOG.md ]] && cat CHANGELOG.md
  } > CHANGELOG.md.new
  mv CHANGELOG.md.new CHANGELOG.md

  ${EDITOR:-vi} CHANGELOG.md

  # Strip the placeholder line if present
  sed -i "/$PLACEHOLDER/d" CHANGELOG.md

  # Check that something was actually written under the version header
  ENTRY=$(sed -n "/^## \[$VERSION\]/,/^## \[/p" CHANGELOG.md | sed '1d;/^## \[/d' | sed '/^$/d')
  if [[ -z "$ENTRY" ]]; then
    echo "Error: no changelog entry for $VERSION (add content under the ## [$VERSION] header)"
    exit 1
  fi
fi

# Bump versions
echo "$VERSION" > VERSION

## Rust crates
cargo set-version "$VERSION"

## JS packages
pnpm -r version "$VERSION" --no-git-tag-version --no-git-checks

## Lua nvim plugin
printf 'return "%s"
' "$VERSION" > editors/nvim/lua/typedown/version.lua

## Nix flake
sed -i "s/version = \"[^\"]*\";/version = \"$VERSION\";/" flake.nix

# Commit and push
git add VERSION Cargo.toml editors/nvim/lua/typedown/version.lua Cargo.lock pnpm-lock.yaml flake.nix
[[ "$BUMP_TYPE" != pre* ]] && git add CHANGELOG.md
find . -name package.json -not -path '*/node_modules/*' -print0 | xargs -0 git add --ignore-errors
find ./crates ./editors -name Cargo.toml -print0 | xargs -0 git add --ignore-errors
git commit -m "chore: release $TAG"
git tag "$TAG"
git push origin HEAD "$TAG"

echo "Pushed tag $TAG"
