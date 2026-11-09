#!/usr/bin/env bash
set -euo pipefail

#
# Helpers
#

# Portable file size in bytes (Linux stat vs macOS stat)
file_bytes() {
  stat --printf="%s" "$1" 2>/dev/null || stat -f "%z" "$1"
}

# Bytes -> KB with one decimal place
to_kb() {
  echo "scale=1; $1 / 1024" | bc
}

# Milliseconds -> seconds with two decimal places
to_seconds() {
  echo "scale=2; $1 / 1000" | bc
}

# Elapsed time in milliseconds between two nanosecond timestamps
elapsed_ms() {
  echo $(( ($2 - $1) / 1000000 ))
}

# Total byte count of all files matching a glob under a directory
glob_size() {
  local dir="$1" pattern="$2"
  find "$dir" -name "$pattern" -exec cat {} + 2>/dev/null | wc -c
}

# Gzipped byte count of all files matching a glob under a directory
glob_gzip_size() {
  local dir="$1" pattern="$2"
  find "$dir" -name "$pattern" -exec cat {} + 2>/dev/null | gzip -c | wc -c
}

# Emit a key=value pair to $GITHUB_OUTPUT
output() {
  echo "$1=$2" >> "$GITHUB_OUTPUT"
}

#
# 1. LSP binary build
#

cargo clean -p typedown-server 2>/dev/null || true

start=$(date +%s%N)
cargo build --locked --release -p typedown-server
end=$(date +%s%N)

lib_build_ms=$(elapsed_ms "$start" "$end")
binary=$(find target/release -maxdepth 1 -name 'typedown-lsp' -type f | head -1)

# Stripped copy reflects what ships to users
stripped="/tmp/typedown-lsp-stripped"
cp "$binary" "$stripped"
strip "$stripped" 2>/dev/null || true

#
# 2. Typerighter npm package build
#

pnpm --filter typerighter @typerighter/rpc-client @typerighter/rpc-server run build 2>/dev/null

#
# 3. SSG build (real-world vault, median of 3 runs)
#

benchmark_vault="examples/huydo862003.github.io"
cd "$benchmark_vault"
pnpm install --frozen-lockfile 2>/dev/null || pnpm install 2>/dev/null
# Link the locally-built typerighter instead of the published version
pnpm link ../../packages/typerighter 2>/dev/null || true

times=()
for iteration in 1 2 3; do
  rm -rf dist
  start=$(date +%s%N)
  pnpm build 2>/dev/null
  end=$(date +%s%N)
  times+=( "$(elapsed_ms "$start" "$end")" )
done

IFS=$'\n' sorted=($(sort -n <<< "${times[*]}")); unset IFS
ssg_build_ms="${sorted[1]}"

# Ensure the final dist/ exists for size measurement
[ -d dist ] || pnpm build 2>/dev/null

#
# 4. Output sizes
#

js_bytes=$(glob_size dist '*.js')
css_bytes=$(glob_size dist '*.css')
html_bytes=$(glob_size dist '*.html')
total_bytes=$(du -sb dist | cut -f1)

js_gz_bytes=$(glob_gzip_size dist '*.js')
css_gz_bytes=$(glob_gzip_size dist '*.css')

page_count=$(find dist -name '*.html' | wc -l)

cd ../..

# npm install size = typerighter dist + rpc-client dist + rpc-server JS + LSP binary
# This reflects what users download when they run `npm install typerighter`
stripped_bytes=$(file_bytes "$stripped")
typerighter_dist=$(du -sb packages/typerighter/dist 2>/dev/null | cut -f1) || typerighter_dist=0
rpc_client_dist=$(du -sb packages/rpc-client/dist 2>/dev/null | cut -f1) || rpc_client_dist=0
rpc_server_js=$(file_bytes packages/rpc-server/install.js 2>/dev/null) || rpc_server_js=0
install_bytes=$(( stripped_bytes + typerighter_dist + rpc_client_dist + rpc_server_js ))

#
# 5. Write GITHUB_OUTPUT
#

output lib_build_s        "$(to_seconds "$lib_build_ms")"
output binary_kb          "$(to_kb "$(file_bytes "$binary")")"
output binary_stripped_kb  "$(to_kb "$(file_bytes "$stripped")")"
output ssg_build_s        "$(to_seconds "$ssg_build_ms")"
output ssg_ms_per_page    "$(echo "scale=1; $ssg_build_ms / $page_count" | bc)"
output total_size_kb      "$(to_kb "$total_bytes")"
output js_kb              "$(to_kb "$js_bytes")"
output js_gz_kb           "$(to_kb "$js_gz_bytes")"
output css_kb             "$(to_kb "$css_bytes")"
output css_gz_kb          "$(to_kb "$css_gz_bytes")"
output html_kb            "$(to_kb "$html_bytes")"
output install_kb         "$(to_kb "$install_bytes")"
output pages              "$page_count"

rm -f "$stripped"
