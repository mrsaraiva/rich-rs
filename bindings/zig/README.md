# rich-rs for Zig

Safe Zig 0.16.0 bindings for the `rich-ffi` native ABI. The Zig package owns no
rendering logic: it checks and projects the generated C ABI while rich-rs does
all parsing and rendering.

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
binding creates no timer, callback, background thread, or cursor-control loop;
the application chooses when to request and display each frame.

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
