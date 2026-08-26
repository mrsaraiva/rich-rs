# `@rich-rs/native`

Complete synchronous Node.js bindings for the `rich-rs` terminal-rendering library. A
C++ Node-API 10 addon projects the stable `rich-ffi` ABI; parsing and rendering stay in
Rust. The supported runtimes are Node.js 24 LTS and Node.js 22.14.0 or newer (CI tests
both release lines).

## Prerequisites and build

Builds require a C++20 compiler, Python and `node-gyp`, Rust when producing the native
library, and a native archive compiled for the same OS/architecture as Node. The package
does not run Cargo or download native inputs during installation, addon build, or module
load. Build the FFI explicitly first:

```bash
cargo build --manifest-path ffi/Cargo.toml
cd bindings/node
npm ci
npm run build -- \
  --rich_ffi_include_dir="$PWD/../../ffi/include" \
  --rich_ffi_static="$PWD/../../ffi/target/debug/librich_ffi.a"
npm test
npm run demo -- --parity
```

The two node-gyp variables always accept explicit absolute paths. Monorepo debug defaults
are platform-aware (`librich_ffi.a`, the macOS archive, or `rich_ffi.lib`) for local work.
Linux static builds additionally link `pthread`, `dl`, and `m`. The root FFI release matrix
produces native static/shared artifacts for Linux GNU x64/ARM64, macOS Apple Silicon, and
Windows MSVC x64/ARM64; pair this source package with the matching artifact. A load error is
deliberately descriptive. Set `RICH_RS_NATIVE_DEBUG=1` only when intentionally loading the
Debug addon.

The checked Node binding CI compiles and exercises Linux x64. The gyp defaults select `.a`
on Linux/macOS and `.lib` on Windows, and encode Linux and Windows system link libraries, but
macOS/Windows addon builds remain target-specific consumer/release-matrix gates rather than a
claim based on this Linux job.

Node-API 10 gives one addon ABI across the maintained Node 22/24 runtime lines. The build
uses C++ exceptions for `node-addon-api`, disables RTTI, enables warnings as errors, and does
not use `NAPI_EXPERIMENTAL`, NAN, direct V8 APIs, or runtime FFI packages.
The addon requires `rich-ffi` ABI version 2 and mechanically inventories all 125 exported
functions from the generated header.

## Ownership, errors, and values

Every native owner has an explicit, idempotent `close()`. Garbage collection is a fallback,
not a scheduling guarantee. A successful builder `finish()` invalidates that builder and
returns a `Renderable`; failure before native ownership transfer leaves the builder live.
Closed or finished access throws `RichError` with `RichErrorCode.InvalidState` locally.

Containers borrow a `Renderable` for the duration of a native call. Rust clones shared
ownership, so the original renderable remains reusable across tables, panels, trees,
columns, alignment and padding, and can be closed after insertion without damaging the
container. Native pointers never appear in JavaScript or TypeScript and objects cannot be
transferred between Workers. Each Node environment gets independent constructors and error
state.

All strings are passed as length-aware UTF-8. Embedded NUL is preserved. Like Node's normal
UTF-8 conversion, an unpaired JavaScript surrogate becomes the Unicode replacement
character. Returned native buffers are copied into JavaScript strings and freed immediately,
including exception paths; no external buffer view or hidden terminator is exposed.

Tree node IDs and progress task IDs are `bigint` exclusively, including valid task ID `0n`.
They are never routed through a JavaScript `number`. Dimension/enum values are checked as
lossless unsigned 32-bit integral numbers. JavaScript shape/type errors use `TypeError` or
`RangeError`; native domain, I/O, render and panic categories use `RichError`. Unknown native
results and ABI mismatches use `RichErrorCode.AbiMismatch`. `ensureAbi()` is invoked by every
constructor/factory and may also be called explicitly; `version()` returns copied UTF-8.

## TypeScript

```ts
import {
  Box,
  Console,
  Justify,
  Measurement,
  Panel,
  Table,
  Text,
  customRenderable,
} from '@rich-rs/native';

const console = new Console().setForceTerminal(false);
const text = new Text('hello').setStyle('bold cyan');
const reusable = text.finish();
const table = new Table().setBox(Box.Rounded).addColumn('value');
table.addRowRenderables([reusable]);
const tableRenderable = table.finish();
const panel = new Panel(tableRenderable).setTitle('rich-rs');
const output = console.render(panel.finish());
```

`index.d.ts` covers every constructor, factory, option, method, enum, ID, error and lifecycle
operation without pointer-shaped numbers or `any`.

## ABI v2 rendering APIs

`Table.grid()` exposes the native borderless grid defaults. `setPadEdge`, `setRowStyles`,
and `addColumnOptions` project the native table configuration atomically; column option
objects reject unknown keys. `Box.None` is valid for tables. `Pretty` is an owning builder
with the same exact-once `finish()`/`close()` state machine as other content builders, and
both `Pretty` and `Syntax` expose `setIndentGuides`.

```ts
const table = Table.grid()
  .setPadEdge(false)
  .setRowStyles(['', 'dim'])
  .setBox(Box.None)
  .addColumnOptions({ ratio: 1, justify: Justify.Left, noWrap: true });
```

`customRenderable({ render, measure? })` installs synchronous JavaScript callbacks behind a
native `Renderable`. `RenderOptions` and `Measurement` are immutable values. A
`SegmentSink` may only be used during the `render` callback that receives it; retained sinks
are tombstoned and fail locally. Callbacks run only in the creating Node environment and OS
thread, are never dispatched to a background thread, and may not reenter any other binding
operation. The addon retains the renderer strongly until the native last `Arc` is released,
catches every callback exception at the C boundary, discards staged segments transactionally,
and restores the original JavaScript exception after the outer render returns.

```ts
const custom = customRenderable({
  measure(options) {
    return new Measurement(1, options.maxWidth);
  },
  render(options, sink) {
    sink.push(`width=${options.maxWidth}`, 'bold cyan').line();
  },
});
```

The repository's `examples/demo.rs` is canonical. `npm run demo -- --parity` recreates that
card through the public high-level API at 120 columns with forced truecolor, performs cold and
warm offscreen renders, and writes the canonical capture. The demo closes every owner in
reverse order and emits no manual terminal control or layout bytes.

## Caller-driven live frames

`Progress`, `Status`, and `Spinner` render one synchronous frame when asked. They create no
timer, callback, cursor loop, background thread, or stored JavaScript value. The caller owns
refresh timing and terminal I/O. A console with terminal output disabled emits neither ANSI
CSI nor OSC control sequences.

## Development and release checks

```bash
npm run format:check
npm run typecheck
npm run package:check
./package-release.sh 1.2.2 /tmp/rich-node-package
```

The package check compares the compiled symbol inventory with all 125 declarations in the
canonical generated header. `package-release.sh` stages source, metadata, the canonical
header and MIT license, excludes host binaries/build output, creates the npm `.tgz`, extracts
it, rebuilds against the supplied matching FFI archive, and loads the resulting consumer.
