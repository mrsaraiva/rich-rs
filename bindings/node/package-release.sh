#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <version> <output-dir>" >&2
  exit 2
fi

version=$1
output_dir=$2
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)
native_archive=${RICH_FFI_STATIC:-$repo_root/ffi/target/debug/librich_ffi.a}
stage_root=$(mktemp -d)
extract_root=$(mktemp -d)
trap 'rm -rf "$stage_root" "$extract_root"' EXIT

if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]]; then
  echo "invalid package version: $version" >&2
  exit 2
fi
if [[ ! -f $repo_root/ffi/include/rich.h ]]; then
  echo "canonical header not found at $repo_root/ffi/include/rich.h" >&2
  exit 1
fi
if [[ ! -f $native_archive ]]; then
  echo "target-matching rich-ffi archive not found at $native_archive" >&2
  exit 1
fi
native_archive=$(realpath "$native_archive")

stage=$stage_root/package
mkdir -p "$stage/src" "$stage/scripts" "$stage/include" "$output_dir"
cp "$script_dir/package.json" "$script_dir/package-lock.json" "$stage/"
cp "$script_dir/.clang-format" "$stage/"
cp "$script_dir/binding.gyp" "$script_dir/index.js" "$script_dir/index.d.ts" "$stage/"
cp "$script_dir/README.md" "$script_dir/package-release.sh" "$stage/"
cp "$script_dir"/src/* "$stage/src/"
cp "$script_dir"/scripts/* "$stage/scripts/"
cp "$repo_root/ffi/include/rich.h" "$stage/include/"
cp "$repo_root/LICENSE" "$stage/"

(cd "$stage" && npm pkg set "version=$version")
(cd "$stage" && npm install --package-lock-only --ignore-scripts --no-audit --no-fund >/dev/null)
(cd "$stage" && npm pack --dry-run --json --ignore-scripts >/dev/null)
archive_name=$(cd "$stage" && npm pack --silent --ignore-scripts \
  --pack-destination "$output_dir")
archive=$output_dir/$archive_name

# npm intentionally omits package-lock.json from packed artifacts. The source
# release contract includes it, so add the exact lock and repack deterministically.
repack_root=$stage_root/repack
mkdir -p "$repack_root"
tar -xzf "$archive" -C "$repack_root"
cp "$stage/package-lock.json" "$repack_root/package/"
tar --sort=name --mtime="@${SOURCE_DATE_EPOCH:-0}" --owner=0 --group=0 \
  --numeric-owner -czf "$archive" -C "$repack_root" package

tar -tzf "$archive" >"$stage_root/archive.list"
for required in package/include/rich.h package/LICENSE package/package-lock.json \
    package/src/addon.cc package/index.d.ts; do
  grep -Fxq "$required" "$stage_root/archive.list" || {
    echo "release archive is missing $required" >&2
    exit 1
  }
done
if grep -E '(^|/)(node_modules|build|coverage)/|\.(node|a|so|dll|dylib|lib)$' \
    "$stage_root/archive.list"; then
  echo "release archive contains a forbidden build/native artifact" >&2
  exit 1
fi

tar -xzf "$archive" -C "$extract_root"
npm ci --ignore-scripts --prefix "$extract_root/package" >/dev/null
npm run build --prefix "$extract_root/package" -- \
  "--rich_ffi_include_dir=$extract_root/package/include" \
  "--rich_ffi_static=$native_archive" >/dev/null
node -e "const rich=require(process.argv[1]); rich.ensureAbi(); if (rich.version() !== '$version') process.exit(1)" \
  "$extract_root/package"

printf '%s\n' "$archive"
