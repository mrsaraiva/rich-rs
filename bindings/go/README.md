# rich-rs for Go

Go 1.27 bindings for the stable `rich-ffi` C ABI. The module is a thin,
idiomatic projection: Rust performs all parsing and rendering. It requires
cgo and a target-matching native `rich-ffi` library; importing or building the
module never runs Cargo and never downloads a native artifact.

## Requirements and module path

- Go 1.27.0 or newer with `CGO_ENABLED=1`.
- The canonical generated `rich.h` and a `rich-ffi` archive built for the
  exact Go target and C toolchain.
- Rust 1.85 or newer only when building the native library from this checkout.

The module path is `github.com/mrsaraiva/rich-rs/bindings/go`. Because it is a
nested module, a future module-proxy release tag must use the nested-module tag
form (for example, `bindings/go/v1.3.0`). No proxy publication policy is made
by this source release.

## Build from the monorepo

Build the FFI explicitly, then give cgo the canonical header and matching
archive. From the repository root on Linux x86_64:

```sh
cargo build --manifest-path ffi/Cargo.toml

export CGO_ENABLED=1
export CGO_CFLAGS="-I$(pwd)/ffi/include"
export CGO_LDFLAGS="$(pwd)/ffi/target/debug/librich_ffi.a"

cd bindings/go
go test ./...
go test -race ./...
go run ./cmd/demo --parity
```

The module's Linux cgo directive supplies the exact native-static library list
reported by this GNU Rust build: `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc`.
On macOS, point `CGO_CFLAGS` at the matching header and
put the matching Darwin `librich_ffi.a` plus any native libraries reported by
the Rust build in `CGO_LDFLAGS`. On Windows, use a cgo C compiler/linker that is
compatible with the selected Rust target and pair it with that target's
`rich_ffi.lib`; add the Windows system libraries reported by the native build.
Do not link a Linux archive on macOS/Windows or silently select a host archive
for a cross target. Source releases are paired with separately produced native
artifacts.

With cgo disabled this package intentionally does not offer a partial or
pure-Go fallback. Enable cgo and provide the native inputs.

## API and ownership

Every native owner has an explicit, idempotent `Close` method. Defer it
immediately after a successful constructor:

```go
console, err := rich.NewConsole()
if err != nil { return err }
defer console.Close()

text, err := rich.NewText("hello")
if err != nil { return err }
defer text.Close()

renderable, err := text.Finish()
if err != nil { return err }
defer renderable.Close()
```

Public wrapper values contain a pointer to shared synchronized state, not a
bare native pointer. Copying a wrapper therefore does not create a second
owner: closing or successfully finishing any copy invalidates every copy, and
concurrent calls/close/finish are serialized without freeing a handle in use.
A runtime finalizer is a last-resort cleanup fallback, not a substitute for
explicit `Close`.

A failed `Finish` leaves the builder live. A successful `Finish` consumes it
and returns a `Renderable`. Composition borrows renderables synchronously;
Rust retains its own shared reference, so one renderable can be inserted into
multiple tables, panels, trees, columns, alignment, or padding values and can
still be rendered independently. It remains the caller's responsibility to
close the original renderable.

Go string bytes are copied to C-owned temporary memory for each call. Embedded
NUL is preserved, and invalid UTF-8 reaches Rust unchanged so it produces the
authoritative `ErrInvalidUTF8`. Native output is copied into a new Go string,
and the producing Rust buffer is always freed—even on native errors, malformed
ABI output, or a Go-side copy failure. Returned strings are wholly Go-owned.

Trees use scalar `NodeID` values and `RootNodeID` (`0`); IDs are meaningful
only while their tree builder is live. Progress uses scalar `TaskID`; task ID
zero is valid. Progress, status, and spinner rendering is caller-driven: this
module starts no goroutine, timer, callback, or cursor-control loop. Custom
renderables are the one scoped callback facility. Native code calls them only
during the invoking thread's synchronous `Console.Render`; it never queues or
background-dispatches them.

## ABI and errors

Every constructor verifies native ABI version 2 before its first native call.
`EnsureABI` can check explicitly, and `Version` returns the synchronized native
package version. The low-level inventory is exhaustive: exactly 125 canonical
`rich.h` functions are compiled and checked by the test suite. Native failures
are `*rich.Error` values with an exact
`ErrorCode`; use `errors.Is(err, rich.ErrInvalidArgument)` and the other
exported sentinels. An unknown native code or malformed output is
`ErrABIMismatch`. A contained Rust panic is returned as `ErrNativePanic`, never
raised as a Go panic.

## Features

The public API covers Console/Renderable/Text/Style, Table/Panel/Tree,
Rule/Columns/Align/Padding, Syntax/Pretty/Markdown/JSON, scoped custom
renderables, and caller-driven Progress/Status/Spinner frames.

`NewGridTable` selects Rust's exact `Table::grid()` defaults. `BoxNone`,
`SetPadEdge`, `SetRowStyles`, and atomic `AddColumnOptions` expose the layout
used by the canonical Rust demo; optional column enum/dimension pointers mean
"use the native default." `NewPretty` owns a copied preformatted Debug/repr
expression, and both Pretty and Syntax expose native indentation guides.

Custom renderers implement a mandatory synchronous method and may implement
the optional measurement interface:

```go
type Swatch struct{}

func (Swatch) Render(options rich.RenderOptions, sink *rich.SegmentSink) error {
    return sink.Push("x", "bold red")
}

func (Swatch) Measure(options rich.RenderOptions) (rich.Measurement, error) {
    return rich.Measurement{Minimum: 1, Maximum: options.MaxWidth}, nil
}

renderable, err := rich.NewCustomRenderable(Swatch{})
```

The sink and options are callback-scoped. Retaining and later using a sink
returns `ErrInvalidState`; invoking any other rich API from render/measure is
also rejected. The binding stores only a numeric `runtime/cgo.Handle` token in
native memory. Native last-Arc destruction deletes it exactly once after any
active callback quiesces, including when containers retain the renderable.
Panics are recovered inside the Go trampoline and become `ErrNativePanic`.
`NewJSONOptions` supplies deliberate JSON defaults; `LineRange` preserves
separate optional syntax endpoints.

## Verification and release source

The canonical demo is a direct port of `examples/demo.rs`. `--parity` forces a
120-column, 72-row, terminal/truecolor configuration and writes only rendered
bytes to stdout; after normalizing its two timing numbers those bytes must
equal `tests/golden/demo-120-truecolor.ansi` exactly.

The normal gates are `gofmt`, `go vet ./...`, `go test ./...`,
`GOEXPERIMENT=cgocheck2 go test ./...`, `go test -race ./...`, the anti-fake
source scan, canonical demo byte comparison, and an external consumer module
using a local `replace`. All require the explicit cgo environment above.

From a repository checkout:

```sh
./package-release.sh 1.3.0 /tmp/rich-go-package
```

This produces `rich-go-1.3.0.tar.gz` with the Go module, tests, README,
canonical header under `include/`, and LICENSE. It intentionally contains no
host native library and must be paired with the exact target FFI artifact.
