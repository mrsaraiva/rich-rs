#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "usage: $0 <version> <output-dir>" >&2
    exit 2
fi

version=$1
output_dir=$2
if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
    echo "invalid version: $version" >&2
    exit 2
fi
if [[ -e $output_dir && ! -d $output_dir ]]; then
    echo "output path is not a directory: $output_dir" >&2
    exit 2
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/../.." && pwd)
header="$repo_root/ffi/include/rich.h"
license="$repo_root/LICENSE"
manifest="$repo_root/Cargo.toml"
if [[ ! -f $header || ! -f $license || ! -f $manifest || ! -f $script_dir/go.mod ]]; then
    echo "package-release.sh must run from a complete rich-rs checkout" >&2
    exit 1
fi
crate_version=$(awk -F '"' '/^version = / { print $2; exit }' "$manifest")
if [[ $version != "$crate_version" ]]; then
    echo "version $version does not match Cargo package version $crate_version" >&2
    exit 2
fi

mkdir -p -- "$output_dir"
output_dir=$(cd -- "$output_dir" && pwd)
archive="$output_dir/rich-go-$version.tar.gz"
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
root="$stage/rich-go-$version"
mkdir -p -- "$root/include" "$root/cmd"

cp -- "$script_dir/go.mod" "$script_dir/README.md" "$script_dir/package-release.sh" "$script_dir/.gitignore" "$root/"
cp -- "$script_dir"/*.go "$root/"
cp -R -- "$script_dir/cmd/demo" "$root/cmd/"
cp -- "$header" "$root/include/rich.h"
cp -- "$license" "$root/LICENSE"

find "$root" -type f \( -name '*.test' -o -name '*.prof' -o -name 'coverage.out' -o -name '*.a' -o -name '*.lib' -o -name '*.so' -o -name '*.dylib' -o -name '*.dll' \) -print -quit | grep -q . && {
    echo "release staging tree contains a build artifact" >&2
    exit 1
}

tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner -C "$stage" -czf "$archive" "rich-go-$version"
while IFS= read -r entry; do
    case "$entry" in
        "rich-go-$version"|"rich-go-$version/"*) ;;
        *) echo "archive contains an invalid root: $entry" >&2; exit 1 ;;
    esac
done < <(tar -tzf "$archive")

sha256sum "$archive"
