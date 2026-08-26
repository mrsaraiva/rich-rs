#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
nonce_file="$(mktemp "${TMPDIR:-/tmp}/rich-ffi-header.XXXXXX")"
trap 'rm -f "$nonce_file"' EXIT
RICH_FFI_HEADER_NONCE="$nonce_file" \
    cargo build --manifest-path "$script_dir/Cargo.toml"
