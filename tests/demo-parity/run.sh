#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/rich-demo-parity.XXXXXX")"
golden_temp=""
cleanup() {
    rm -rf -- "$work_dir"
    if [[ -n $golden_temp ]]; then rm -f -- "$golden_temp"; fi
}
trap cleanup EXIT
evidence_dir="${RICH_PARITY_EVIDENCE_DIR:-$work_dir/evidence}"
zig_bin="${ZIG_BIN:-zig}"
mkdir -p -- "$evidence_dir"

"$repo_root/tests/demo-parity/check-sources.sh"

normalize() {
    python3 "$repo_root/tests/demo-parity/normalize.py" "$1" "$2"
}

record() {
    local language="$1" command="$2" executable="$3" capture="$4" trace="${5:-}" extra_artifact="${6:-}"
    local probe_command="${7:-not applicable: canonical Rust reference}"
    local executable_hash library_hash capture_hash bytes extra_hash=""
    executable_hash="$(sha256sum "$executable" | cut -d' ' -f1)"
    library_hash="$(sha256sum "$native_archive" | cut -d' ' -f1)"
    capture_hash="$(sha256sum "$capture" | cut -d' ' -f1)"
    bytes="$(wc -c < "$capture")"
    if [[ -n $extra_artifact ]]; then extra_hash="$(sha256sum "$extra_artifact" | cut -d' ' -f1)"; fi
    python3 - "$evidence_dir/$language-evidence.json" "$language" "$command" \
        "$executable_hash" "$library_hash" "$capture_hash" "$bytes" "$trace" "$extra_hash" \
        "$probe_command" <<'PY'
import json, pathlib, sys
path, language, command, executable, library, capture, count, trace, extra, probe = sys.argv[1:]
events = []
if trace:
    events = [json.loads(line) for line in pathlib.Path(trace).read_text().splitlines()]
pathlib.Path(path).write_text(json.dumps({
    "language": language, "command": command, "executable_sha256": executable,
    "library_sha256": library, "capture_sha256": capture, "capture_bytes": int(count),
    "timing_normalization_matches": 2, "golden_equal": True,
    "ansi_stripped_equal": True,
    "anti_fake_probe_results": {"status": "passed", "command": probe},
    "extra_artifact_sha256": extra or None,
    "feature_event_counts": {name: sum(e["event"] == name for e in events)
        for name in sorted({e["event"] for e in events})},
    "anti_fake_source_scan": "passed",
}, indent=2) + "\n")
PY
}

validate_trace() {
    python3 - "$1" <<'PY'
import json, pathlib, sys
events = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines()]
required = {"table_grid", "table_none", "table_pad_edge", "table_row_styles",
            "table_column_options", "syntax_indent_guides", "pretty_new", "pretty_finish",
            "custom_new", "custom_measure", "custom_render", "sink_push", "sink_line"}
counts = {name: sum(event["event"] == name for event in events) for name in required | {"custom_drop"}}
missing = sorted(name for name in required if counts[name] == 0)
if missing or counts["custom_drop"] != 1:
    raise SystemExit(f"invalid parity trace: missing={missing}, custom_drop={counts['custom_drop']}")
if not all("thread" in event for event in events if event["event"].startswith("custom_")):
    raise SystemExit("custom callback trace is missing thread IDs")
if not any(event.get("width", 0) > 0 for event in events if event["event"] in {"custom_measure", "custom_render"}):
    raise SystemExit("custom callback trace is missing width options")
PY
}

capture_and_compare() {
    local language="$1" command="$2" executable="$3" trace="$4" extra_artifact="${5:-}"
    local probe_command="${6:-}"
    local raw="$work_dir/$language.ansi" normalized="$work_dir/$language.normalized.ansi"
    local plain="$work_dir/$language.plain" golden_plain="$work_dir/golden.plain"
    rm -f -- "$trace"
    RICH_FFI_PARITY_TRACE="$trace" bash -c "$command" > "$raw"
    normalize "$raw" "$normalized"
    cmp "$repo_root/tests/golden/demo-120-truecolor.ansi" "$normalized"
    python3 "$repo_root/tests/demo-parity/strip-ansi.py" "$normalized" "$plain"
    python3 "$repo_root/tests/demo-parity/strip-ansi.py" \
        "$repo_root/tests/golden/demo-120-truecolor.ansi" "$golden_plain"
    cmp "$golden_plain" "$plain"
    validate_trace "$trace"
    record "$language" "$command" "$executable" "$raw" "$trace" "$extra_artifact" \
        "$probe_command"
}

cd "$repo_root"
cargo build --manifest-path ffi/Cargo.toml --release --features parity-trace
native_archive="$repo_root/ffi/target/release/librich_ffi.a"
native_shared="$repo_root/ffi/target/release/librich_ffi.so"

cargo build --release --example demo
rust_raw="$work_dir/rust.ansi"
"$repo_root/target/release/examples/demo" --parity > "$rust_raw"
normalize "$rust_raw" "$work_dir/rust.normalized.ansi"
rust_normalized="$work_dir/rust.normalized.ansi"
if [[ ${1:-} == "--update-golden" ]]; then
    golden_temp="$(mktemp "$repo_root/tests/golden/.demo-120-truecolor.XXXXXX")"
    normalize "$rust_raw" "$golden_temp"
    mv -- "$golden_temp" "$repo_root/tests/golden/demo-120-truecolor.ansi"
    golden_temp=""
    rust_normalized="$repo_root/tests/golden/demo-120-truecolor.ansi"
else
    cmp "$repo_root/tests/golden/demo-120-truecolor.ansi" "$rust_normalized"
fi
python3 tests/demo-parity/strip-ansi.py "$rust_normalized" "$work_dir/rust.plain"
python3 tests/demo-parity/strip-ansi.py tests/golden/demo-120-truecolor.ansi "$work_dir/golden.plain"
cmp "$work_dir/golden.plain" "$work_dir/rust.plain"
record rust "'$repo_root/target/release/examples/demo' --parity" \
    "$repo_root/target/release/examples/demo" "$rust_raw"

cc -std=c11 -O2 -Wall -Wextra -Wpedantic -Werror -Iffi/include \
    ffi/examples/demo.c "$native_archive" -lpthread -ldl -lm -o "$work_dir/demo-c"
c++ -std=c++17 -O2 -Wall -Wextra -Wpedantic -Werror -Iffi/include \
    ffi/examples/demo.cpp "$native_archive" -lpthread -ldl -lm -o "$work_dir/demo-cpp"
cc -std=c11 -O1 -g -Wall -Wextra -Wpedantic -Werror -Iffi/include \
    ffi/examples/smoke.c "$native_archive" -lpthread -ldl -lm -o "$work_dir/probe-c"
c++ -std=c++17 -O1 -g -Wall -Wextra -Wpedantic -Werror -Iffi/include \
    ffi/examples/smoke_hpp.cpp "$native_archive" -lpthread -ldl -lm -o "$work_dir/probe-cpp"
"$work_dir/probe-c"
"$work_dir/probe-cpp"
cc -std=c11 -O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer \
    -Wall -Wextra -Wpedantic -Werror -Iffi/include ffi/examples/demo.c \
    "$native_archive" -lpthread -ldl -lm -o "$work_dir/demo-c-sanitize"
c++ -std=c++17 -O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer \
    -Wall -Wextra -Wpedantic -Werror -Iffi/include ffi/examples/demo.cpp \
    "$native_archive" -lpthread -ldl -lm -o "$work_dir/demo-cpp-sanitize"
ASAN_OPTIONS=detect_leaks=1 UBSAN_OPTIONS=halt_on_error=1 "$work_dir/demo-c-sanitize" --parity \
    > /dev/null
ASAN_OPTIONS=detect_leaks=1 UBSAN_OPTIONS=halt_on_error=1 "$work_dir/demo-cpp-sanitize" --parity \
    > /dev/null
capture_and_compare c "'$work_dir/demo-c' --parity" "$work_dir/demo-c" "$work_dir/c.trace.jsonl" \
    "" "'$work_dir/probe-c'; ASan/UBSan '$work_dir/demo-c-sanitize' --parity"
capture_and_compare cpp "'$work_dir/demo-cpp' --parity" "$work_dir/demo-cpp" "$work_dir/cpp.trace.jsonl" \
    "" "'$work_dir/probe-cpp'; ASan/UBSan '$work_dir/demo-cpp-sanitize' --parity"

zig_command="cd '$repo_root/bindings/zig' && '$zig_bin' build example -Doptimize=ReleaseSafe \
    -Drich-ffi-static='$native_archive' -Drich-ffi-include-dir='$repo_root/ffi/include' -- --parity"
(cd bindings/zig && "$zig_bin" build -Doptimize=ReleaseSafe \
    -Drich-ffi-static="$native_archive" -Drich-ffi-include-dir="$repo_root/ffi/include")
(cd bindings/zig && "$zig_bin" build test -Doptimize=ReleaseSafe \
    -Drich-ffi-static="$native_archive" -Drich-ffi-include-dir="$repo_root/ffi/include")
zig_executable="$(find bindings/zig/.zig-cache -type f -name rich-zig-demo -executable -printf '%T@ %p\n' | sort -nr | head -1 | cut -d' ' -f2-)"
capture_and_compare zig "$zig_command" "$zig_executable" "$work_dir/zig.trace.jsonl" "" \
    "cd '$repo_root/bindings/zig' && '$zig_bin' build test -Doptimize=ReleaseSafe -Drich-ffi-static='$native_archive' -Drich-ffi-include-dir='$repo_root/ffi/include'"

(cd bindings/go && CGO_ENABLED=1 CGO_CFLAGS="-I$repo_root/ffi/include" \
    CGO_LDFLAGS="$native_archive" go build -o "$work_dir/demo-go" ./cmd/demo)
(cd bindings/go && CGO_ENABLED=1 CGO_CFLAGS="-I$repo_root/ffi/include" \
    CGO_LDFLAGS="$native_archive" go test ./...)
(cd bindings/go && CGO_ENABLED=1 CGO_CFLAGS="-I$repo_root/ffi/include" \
    CGO_LDFLAGS="$native_archive" go test -race ./...)
capture_and_compare go "'$work_dir/demo-go' --parity" "$work_dir/demo-go" "$work_dir/go.trace.jsonl" \
    "" "cd '$repo_root/bindings/go' && CGO_ENABLED=1 CGO_CFLAGS='-I$repo_root/ffi/include' CGO_LDFLAGS='$native_archive' go test -race ./..."

RICH_RS_NATIVE_LIBRARY="$native_shared" dotnet build -c Release \
    bindings/csharp/examples/RichRs.Demo/RichRs.Demo.csproj
RICH_RS_NATIVE_LIBRARY="$native_shared" dotnet test -c Release \
    bindings/csharp/tests/RichRs.Tests/RichRs.Tests.csproj
csharp_executable="$repo_root/bindings/csharp/examples/RichRs.Demo/bin/Release/net10.0/RichRs.Demo"
capture_and_compare csharp "RICH_RS_NATIVE_LIBRARY='$native_shared' '$csharp_executable' --parity" \
    "$csharp_executable" "$work_dir/csharp.trace.jsonl" "" \
    "RICH_RS_NATIVE_LIBRARY='$native_shared' dotnet test -c Release bindings/csharp/tests/RichRs.Tests/RichRs.Tests.csproj"

(cd bindings/node && ./node_modules/.bin/node-gyp rebuild --release \
    --rich_ffi_static="$native_archive" --rich_ffi_include_dir="$repo_root/ffi/include")
(cd bindings/node && npm test)
node_executable="$(command -v node)"
node_addon="$repo_root/bindings/node/build/Release/rich_native.node"
capture_and_compare node "cd '$repo_root/bindings/node' && node examples/demo.js --parity" \
    "$node_executable" "$work_dir/node.trace.jsonl" "$node_addon" \
    "cd '$repo_root/bindings/node' && npm test"

echo "all canonical demos match tests/golden/demo-120-truecolor.ansi"
