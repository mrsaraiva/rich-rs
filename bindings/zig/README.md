# rich-rs for Zig

Safe Zig 0.16.0 bindings for the `rich-ffi` ABI version 2. The Zig package owns no
rendering logic: it checks and projects the generated C ABI while rich-rs does
all parsing and rendering. Its low-level import exhaustively checks the frozen
125-function inventory.

## Requirements

- Zig 0.16.0 exactly.
- A target-matching `rich-ffi` static library and the generated `rich.h`.
- Rust 1.85 or newer only when building `rich-ffi` from a source checkout.

The build does not run Cargo, download Zig, or fetch a native library. Source
checkout users build the native library explicitly:

```sh
cargo build --manifest-path ffi/Cargo.toml

zig build --build-file bindings/zig/build.zig test \
  -Drich-ffi-include-dir=../../ffi/include \
  -Drich-ffi-static=../../ffi/target/debug/librich_ffi.a
zig build --build-file bindings/zig/build.zig example \
  -Drich-ffi-include-dir=../../ffi/include \
  -Drich-ffi-static=../../ffi/target/debug/librich_ffi.a
zig build --build-file bindings/zig/build.zig check \
  -Drich-ffi-include-dir=../../ffi/include \
  -Drich-ffi-static=../../ffi/target/debug/librich_ffi.a
```

The example is the Zig projection of the canonical Rust demo. Its deterministic
120-column forced-terminal mode is:

```sh
zig build --build-file bindings/zig/build.zig example -- --parity
```

In parity mode it uses native table layout, Pretty, Syntax indentation guides,
and a synchronous Zig-defined ColorBox renderable. Only the two measured timing
numbers vary; the repository parity runner requires every other output byte to
match the Rust demo.

Paths supplied with `-Drich-ffi-include-dir` and `-Drich-ffi-static` are
resolved from the Zig package root; absolute paths are accepted. The defaults
select `../../ffi/include` and the debug archive under `../../ffi/target` for
monorepo development. A packaged release defaults to its bundled `include`
directory. Windows defaults to `rich_ffi.lib`; other targets default to
`librich_ffi.a`. An explicit path always wins, and cross-target builds require
one so the host archive can never be selected accidentally.

## Add it to an application

Add the Zig source archive to the application's `build.zig.zon` (replace the
URL, version, and package hash with values from the release):

```zig
.dependencies = .{
    .rich = .{
        .url = "https://github.com/mrsaraiva/rich-rs/releases/download/v1.2.2/rich-zig-1.2.2.tar.gz",
        .hash = "rich-1.2.2-<zig-package-hash>",
    },
},
```

Then pass the target-specific FFI artifact to the dependency and import its
public `rich` module:

```zig
const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const dep = b.dependency("rich", .{
        .target = target,
        .optimize = optimize,
        .@"rich-ffi-include-dir" = b.pathFromRoot("vendor/rich-ffi/include"),
        .@"rich-ffi-static" = b.pathFromRoot("vendor/rich-ffi/lib/librich_ffi.a"),
    });
    const exe = b.addExecutable(.{
        .name = "app",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
            .imports = &.{.{ .name = "rich", .module = dep.module("rich") }},
        }),
    });
    b.installArtifact(exe);
}
```

The archive filename in that example is for Unix. Select the archive from the
matching `rich-ffi-<version>-<target>.tar.gz`; use `rich_ffi.lib` for Windows.
Never link a native archive built for a different target.

## Ownership rules

All handle-owning values are non-copyable by contract. Keep each live owner in
one `var`, pass it by pointer, and call its idempotent `deinit` exactly through
that variable:

```zig
var console = try rich.Console.init();
defer console.deinit();

const output = try console.renderMarkup(allocator, "[bold]hello[/bold]");
defer allocator.free(output);
```

Successful builder `finish` calls invalidate the builder and return a new
`Renderable`; failed finishes leave the builder live so it can still be
cleaned up. Returned byte slices belong to the caller's allocator and must be
freed with that same allocator. The wrapper always releases the intermediate
native `RichBuffer`, including if the Zig allocation fails.

Composition borrows `*const Renderable`. Containers retain the native shared
renderable internally, so the original renderable remains independently usable
and must still be deinitialized by its owner. Do not copy a live owner struct.

Trees use scalar `NodeId` values, with `rich.rootNode` (`0`) naming the root.
Node IDs contain no native pointer and are useful only with the still-live
`Tree` that created them.

`Progress`, `Status`, and `Spinner` are caller-driven frame renderers. The Zig
binding creates no timer, background thread, or cursor-control loop;
the application chooses when to request and display each frame.

## ABI v2 layout and content

`Table.initGrid`, `setPadEdge`, `setRowStyles`, and `addColumnOptions` project
the native layout engine directly. `ColumnOptions` uses optionals for justify,
vertical alignment, fixed width, minimum width, and ratio; `null` means the
native Column default. `.none` is valid for `Table.setBox` and intentionally
remains invalid for Panel. Headers and style slices are borrowed only for the
call and copied/retained by native code.

`Pretty` is an ordinary owning builder with `init`, `setIndentGuides`, `finish`,
and idempotent `deinit`. `Syntax.setIndentGuides` enables the same native guide
renderer. Neither feature is implemented with JSON or host-side highlighting.

## Scoped custom renderables

`customRenderable(context, callbacks)` creates an Arc-backed native renderable.
The render callback is required; measure and drop are optional. A non-null
context requires drop. On successful construction its ownership transfers to
native code, and the drop callback runs exactly once when the last reference
retained by any container is released. On construction failure it remains with
the caller. Last release may occur on any thread that owns a retained native
reference, so the callback state, user context, and drop callback must all be
safe to access and destroy on that thread.

Callbacks run synchronously on the thread executing `Console.render`; native
code never queues or background-dispatches them. Calls for one adapter are
serialized. The callback receives copied `RenderOptions` and a scoped
`SegmentSink`. `push` and `line` copy their inputs into native transactional
storage. The wrapper tombstones the sink when the callback returns, so retained
or wrong-thread uses fail locally with `error.InvalidState`. Do not copy live
sink/owner structs, panic from a callback, re-enter rich APIs, or destroy rich
owners inside a callback. The wrapper's small callback-state record uses the C
allocator and is released by its native last-reference drop trampoline.

## Native targets

Release-native artifacts are built for:

| Target | Static library |
|---|---|
| `x86_64-unknown-linux-gnu` | `librich_ffi.a` |
| `aarch64-unknown-linux-gnu` | `librich_ffi.a` |
| `aarch64-apple-darwin` | `librich_ffi.a` |
| `x86_64-pc-windows-msvc` | `rich_ffi.lib` |
| `aarch64-pc-windows-msvc` | `rich_ffi.lib` |

The Zig workflow executes tests and the demo on Linux x86_64. The existing
native FFI workflow builds the other targets on native runners; the Zig build
must be given the corresponding archive for those targets. There is currently
no implicit native download, universal archive, or claim that a Linux `.a` can
be used for macOS or Windows.
