#!/usr/bin/env bash
set -euo pipefail

# Lib build

# Clean and measure release build time
cargo clean -p typedown-server 2>/dev/null || true
start=$(date +%s%N)
cargo build --locked --release -p typedown-server
end=$(date +%s%N)
lib_build_ms=$(( (end - start) / 1000000 ))
lib_build_s=$(echo "scale=2; $lib_build_ms / 1000" | bc)

# Binary size
binary=$(find target/release -maxdepth 1 -name 'typedown-server' -type f | head -1)
binary_size=$(stat --printf="%s" "$binary" 2>/dev/null || stat -f "%z" "$binary")
binary_kb=$(echo "scale=1; $binary_size / 1024" | bc)

# Typerighter package build

cd packages/typerighter
pnpm build 2>/dev/null
cd ../..

# Homepage SSG build (median of 3)

cd homepage
times=()
for i in 1 2 3; do
  rm -rf dist
  start=$(date +%s%N)
  pnpm build 2>/dev/null
  end=$(date +%s%N)
  elapsed=$(( (end - start) / 1000000 ))
  times+=($elapsed)
done

IFS=$'\n' sorted=($(sort -n <<< "${times[*]}")); unset IFS
ssg_build_ms=${sorted[1]}
ssg_build_s=$(echo "scale=2; $ssg_build_ms / 1000" | bc)

# Ensure final build exists
if [ ! -d dist ]; then
  pnpm build 2>/dev/null
fi

# Output sizes

total_size=$(du -sb dist | cut -f1)
total_size_kb=$(echo "scale=1; $total_size / 1024" | bc)

js_size=$(find dist -name '*.js' -exec cat {} + 2>/dev/null | wc -c)
css_size=$(find dist -name '*.css' -exec cat {} + 2>/dev/null | wc -c)
html_size=$(find dist -name '*.html' -exec cat {} + 2>/dev/null | wc -c)

js_kb=$(echo "scale=1; $js_size / 1024" | bc)
css_kb=$(echo "scale=1; $css_size / 1024" | bc)
html_kb=$(echo "scale=1; $html_size / 1024" | bc)

js_gz_size=$(find dist -name '*.js' -exec cat {} + 2>/dev/null | gzip -c | wc -c)
js_gz_kb=$(echo "scale=1; $js_gz_size / 1024" | bc)

pages=$(find dist -name '*.html' | wc -l)

cd ..

# Write outputs

{
  echo "lib_build_s=$lib_build_s"
  echo "binary_kb=$binary_kb"
  echo "ssg_build_s=$ssg_build_s"
  echo "total_size_kb=$total_size_kb"
  echo "js_kb=$js_kb"
  echo "js_gz_kb=$js_gz_kb"
  echo "css_kb=$css_kb"
  echo "html_kb=$html_kb"
  echo "pages=$pages"
} >> "$GITHUB_OUTPUT"
