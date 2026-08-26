#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
python3 - "$repo_root" <<'PY'
import json
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
sources = {
    "c": root / "ffi/examples/demo.c",
    "cpp": root / "ffi/examples/demo.cpp",
    "zig": root / "bindings/zig/examples/demo.zig",
    "go": root / "bindings/go/cmd/demo/main.go",
    "csharp": root / "bindings/csharp/examples/RichRs.Demo/Program.cs",
    "node": root / "bindings/node/examples/demo.js",
}
for path in sources.values():
    if not path.is_file():
        raise SystemExit(f"missing canonical demo source: {path.relative_to(root)}")

spellings = [r"\x1b", r"\x1B", r"\033", r"\e", r"\u001b", r"\u001B", r"\u{1b}", r"\u{1B}"]
common = ["demo-120-truecolor.ansi", "tests/golden", "--update-golden", "@embedFile", "include_bytes!"]
decode_apis = [
    r"(?:base64|hex)[._-]?(?:decode|decoder)", r"FromBase64String", r"fromBase64",
    r"Buffer\.from\([^\n]*(?:base64|hex)", r"Encoding\.Hex", r"hex\.Decode",
]
static_arrays = {
    "c": [r"\bstatic\s+(?:const\s+)?(?:unsigned\s+char|uint8_t|char)\s+\w+\s*\[[^]]*\]\s*=\s*\{"],
    "cpp": [r"\bstatic\s+(?:const\s+)?(?:std::)?(?:byte|uint8_t|unsigned\s+char|char)\s+\w+\s*\[[^]]*\]\s*=\s*\{"],
    "zig": [r"\bconst\s+\w+\s*:\s*\[\d+\]u8\s*=\s*\.\{"],
    "go": [r"\bvar\s+\w+\s*=\s*\[\]byte\s*\{"],
    "csharp": [r"\bstatic\s+(?:readonly\s+)?byte\s*\[\]\s+\w+\s*=\s*(?:new\s+byte\s*\[\]\s*)?\{"],
    "node": [r"\bBuffer\.from\s*\(\s*\["],
}
language_forbidden = {
    "c": [r"\bsystem\s*\(", r"\bpopen\s*\(", r"\b_popen\s*\(", r"\bexec\w*\s*\(", r"\bposix_spawn\w*\s*\(", r"\bCreateProcess\w*\s*\(", r"\bfopen\s*\(", r"\bopen\s*\("],
    "cpp": [r"\bsystem\s*\(", r"\bpopen\s*\(", r"\b_popen\s*\(", r"\bexec\w*\s*\(", r"\bposix_spawn\w*\s*\(", r"\bCreateProcess\w*\s*\(", r"\bfopen\s*\(", r"\bopen\s*\("],
    "zig": [r"std\.process\.(?:Child|run)", r"std\.(?:fs|http)"],
    "go": [r'"(?:os/exec|net(?:/[^\"]*)?)"'],
    "csharp": [r"System\.IO\.File", r"\bFileStream\b", r"\bProcess\b", r"\bHttpClient\b"],
    "node": [r"child_process", r"\b(?:spawn|exec)\w*\s*\("],
}
for language, path in sources.items():
    text = path.read_text(encoding="utf-8")
    if "\x1b" in text or any(0x2500 <= ord(char) <= 0x257F for char in text):
        raise SystemExit(f"forbidden terminal byte/box glyph in {path.relative_to(root)}")
    for token in spellings + common:
        if token in text:
            raise SystemExit(f"forbidden token {token!r} in {path.relative_to(root)}")
    for pattern in language_forbidden[language]:
        if re.search(pattern, text):
            raise SystemExit(f"forbidden API /{pattern}/ in {path.relative_to(root)}")
    for pattern in decode_apis + static_arrays[language]:
        if re.search(pattern, text, re.I):
            raise SystemExit(f"forbidden stored-output decoder/array in {path.relative_to(root)}")
    glyph_count = text.count("▄") + sum(text.count(value) for value in (r"\u2584", r"\u{2584}", r"\xE2\x96\x84"))
    if glyph_count != 1:
        raise SystemExit(f"{path.relative_to(root)} must contain exactly one half-block input (found {glyph_count})")

allow = {
    "c": {"rich.h", "assert.h", "math.h", "stdint.h", "stdio.h", "stdlib.h", "string.h", "time.h"},
    "cpp": {"rich.hpp", "algorithm", "array", "cassert", "chrono", "cmath", "cstdint", "cstdio", "cstdlib", "functional", "iomanip", "iostream", "memory", "optional", "sstream", "string", "string_view", "utility", "vector"},
}
for language in ("c", "cpp"):
    includes = set(re.findall(r'^\s*#include\s*[<"]([^>"]+)[>"]', sources[language].read_text(), re.M))
    if not includes <= allow[language]:
        raise SystemExit(f"unexpected {language} includes: {sorted(includes - allow[language])}")

go_imports = set(re.findall(r'^\s*"([^"]+)"', sources["go"].read_text(), re.M))
go_allow = {"bytes", "fmt", "math", "os", "strings", "time", "github.com/mrsaraiva/rich-rs/bindings/go"}
if not go_imports <= go_allow:
    raise SystemExit(f"unexpected Go imports: {sorted(go_imports - go_allow)}")

zig_imports = set(re.findall(r'@import\("([^"]+)"\)', sources["zig"].read_text()))
if zig_imports != {"std", "rich"}:
    raise SystemExit(f"unexpected Zig imports: {sorted(zig_imports)}")

csharp_imports = set(re.findall(r'^using\s+([\w.]+)\s*;', sources["csharp"].read_text(), re.M))
csharp_allow = {"RichRs", "System", "System.Collections.Generic", "System.Diagnostics", "System.Globalization", "System.Text"}
if not csharp_imports <= csharp_allow or "RichRs" not in csharp_imports:
    raise SystemExit(f"unexpected C# imports: {sorted(csharp_imports - csharp_allow)}")

node_text = sources["node"].read_text()
node_imports = set(re.findall(r"require\(['\"]([^'\"]+)['\"]\)", node_text))
node_imports.update(re.findall(r"from\s+['\"]([^'\"]+)['\"]", node_text))
if not node_imports <= {"..", "node:perf_hooks"} or ".." not in node_imports:
    raise SystemExit(f"unexpected Node imports: {sorted(node_imports)}")
if "require" in (root / "bindings/go/go.mod").read_text():
    raise SystemExit("bindings/go/go.mod must not contain require directives")

package = json.loads((root / "bindings/node/package.json").read_text())
if set(package.get("dependencies", {})) != {"node-addon-api"}:
    raise SystemExit("Node production dependency allowlist mismatch")
if set(package.get("devDependencies", {})) != {"@types/node", "node-gyp", "typescript"}:
    raise SystemExit("Node development dependency allowlist mismatch")
if package.get("scripts", {}).get("demo") != "node examples/demo.js":
    raise SystemExit("Node demo script mismatch")

project = (root / "bindings/csharp/examples/RichRs.Demo/RichRs.Demo.csproj").read_text()
if project.count("<ProjectReference") != 1 or "<PackageReference" in project:
    raise SystemExit("C# demo dependency allowlist mismatch")
if ".dependencies = .{}" not in (root / "bindings/zig/build.zig.zon").read_text():
    raise SystemExit("Zig package dependencies must be empty")
PY
