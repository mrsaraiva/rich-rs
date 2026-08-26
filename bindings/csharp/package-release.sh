#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "usage: $0 <version> <output-dir>" >&2
    exit 2
fi

version=$1
output_dir=$2
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(cd -- "$script_dir/../.." && pwd)
project="$script_dir/src/RichRs/RichRs.csproj"
demo="$script_dir/examples/RichRs.Demo/RichRs.Demo.csproj"

if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "version must be a stable SemVer triplet" >&2
    exit 2
fi
if [[ -z ${RICH_RS_NATIVE_LIBRARY:-} || ! -f ${RICH_RS_NATIVE_LIBRARY:-} ]]; then
    echo "RICH_RS_NATIVE_LIBRARY must name the target-matching rich-ffi shared library" >&2
    exit 2
fi

managed_version=$(sed -n 's:.*<Version>\([^<]*\)</Version>.*:\1:p' "$script_dir/Directory.Build.props")
root_version=$(sed -n '0,/^version = /s/^version = "\([^"]*\)"/\1/p' "$repository_root/Cargo.toml")
ffi_version=$(sed -n '0,/^version = /s/^version = "\([^"]*\)"/\1/p' "$repository_root/ffi/Cargo.toml")
native_version=$(dotnet run --project "$demo" --configuration Release --no-build -- --version-only)

for observed in "$managed_version" "$root_version" "$ffi_version" "$native_version"; do
    if [[ $observed != "$version" ]]; then
        echo "version drift: requested $version but observed $observed" >&2
        exit 1
    fi
done

mkdir -p -- "$output_dir"
find "$output_dir" -maxdepth 1 -type f \( -name 'RichRs.Native.*.nupkg' -o -name 'RichRs.Native.*.snupkg' \) -delete
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-315532800}
dotnet pack "$project" \
    --configuration Release \
    --no-restore \
    --output "$output_dir" \
    -p:PackageVersion="$version" \
    -p:ContinuousIntegrationBuild=true

package="$output_dir/RichRs.Native.$version.nupkg"
test -f "$package"

# NuGet writes random relationship IDs and a random core-properties filename.
# Normalize those values, timestamps, ZIP metadata, and entry order so identical
# sources produce byte-identical release artifacts.
normalize_dir=$(mktemp -d)
trap 'rm -rf -- "$normalize_dir"' EXIT
unzip -q "$package" -d "$normalize_dir"
core_dir="$normalize_dir/package/services/metadata/core-properties"
core_file=$(find "$core_dir" -maxdepth 1 -type f -name '*.psmdcp' -print -quit)
mv -- "$core_file" "$core_dir/richrs.psmdcp"
sed -E -i \
    -e 's#Target="/package/services/metadata/core-properties/[^"]+"#Target="/package/services/metadata/core-properties/richrs.psmdcp"#' \
    -e '/manifest/ s#Id="R[0-9A-F]+"#Id="RICHRS_MANIFEST"#' \
    -e '/metadata\/core-properties/ s#Id="R[0-9A-F]+"#Id="RICHRS_METADATA"#' \
    "$normalize_dir/_rels/.rels"
find "$normalize_dir" -type f -exec touch -d "@$SOURCE_DATE_EPOCH" {} +
normalized="$normalize_dir/normalized.nupkg"
(
    cd "$normalize_dir"
    find . -type f ! -name normalized.nupkg -printf '%P\0' \
        | LC_ALL=C sort -z \
        | xargs -0 zip -X -q "$normalized"
)
mv -- "$normalized" "$package"

contents=$(unzip -Z1 "$package")
for required in \
    "lib/net10.0/RichRs.dll" \
    "lib/net10.0/RichRs.xml" \
    "README.md" \
    "LICENSE"; do
    if ! grep -Fxq "$required" <<<"$contents"; then
        echo "package is missing $required" >&2
        exit 1
    fi
done
if grep -E '(^|/)(runtimes|native)/|\.(so|dylib|dll)$' <<<"$contents" | grep -vF 'lib/net10.0/RichRs.dll' >/dev/null; then
    echo "managed package must not contain native runtime assets" >&2
    exit 1
fi

printf '%s\n' "$package"
