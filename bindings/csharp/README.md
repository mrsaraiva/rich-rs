# rich-rs for .NET

`RichRs.Native` is the safe .NET 10 / C# 14 projection of the `rich-ffi`
native ABI. Rust remains the only rendering implementation. The managed
package checks ABI version 2 and its exact 125-symbol inventory, converts native result codes to typed
exceptions, and exposes every native family without exposing raw pointers.

## Requirements and native loading

- .NET 10 LTS.
- A target-matching `rich-ffi` shared library at version 1.2.2.

The NuGet package intentionally contains only managed assemblies, XML API
documentation, and metadata. Pair it with the matching
`rich-ffi-1.2.2-<target>` release artifact. A Linux library cannot be used on
macOS or Windows, and architecture must match the process.

The resolver first honors an absolute `RICH_RS_NATIVE_LIBRARY` path. Otherwise
it uses the platform loader to find `rich_ffi.dll`, `librich_ffi.so`, or
`librich_ffi.dylib`. A failed load explains how to set the override:

```sh
export RICH_RS_NATIVE_LIBRARY=/absolute/path/to/librich_ffi.so
```

Normal restore, build, and package loading never run Cargo, download native
code, or guess a runtime identifier.

## Build, test, demo, and pack

From the repository root:

```sh
cargo build --manifest-path ffi/Cargo.toml
dotnet restore bindings/csharp/RichRs.sln --locked-mode
dotnet format bindings/csharp/RichRs.sln --verify-no-changes --no-restore
dotnet build bindings/csharp/RichRs.sln --configuration Release --no-restore

RICH_RS_NATIVE_LIBRARY="$PWD/ffi/target/debug/librich_ffi.so" \
  dotnet test bindings/csharp/RichRs.sln --configuration Release --no-build
RICH_RS_NATIVE_LIBRARY="$PWD/ffi/target/debug/librich_ffi.so" \
  dotnet run --project bindings/csharp/examples/RichRs.Demo \
    --configuration Release --no-build -- --parity

RICH_RS_NATIVE_LIBRARY="$PWD/ffi/target/debug/librich_ffi.so" \
  bindings/csharp/package-release.sh 1.2.2 /tmp/rich-csharp-package
```

On Windows, set the override to `rich_ffi.dll`; on macOS, use
`librich_ffi.dylib`.

## Usage

All owners are sealed `IDisposable` classes backed by a family-specific
`SafeHandle`. Dispose deterministically with `using`; repeated disposal is a
no-op, while use after disposal or successful `Finish()` throws
`ObjectDisposedException`.

```csharp
using RichRs;

using RichConsole console = new();
console.ForceTerminal = false;
console.SetSize(80, 24);

using Text text = new("Reusable native content");
text.SetStyle("bold cyan");
using Renderable reusable = text.Finish();

using Table table = new();
table.SetTitle("rich-rs from .NET");
table.AddColumn(reusable);
table.AddColumn("State");
table.AddRow(["borrowed", "safe"]);
using Renderable tableRenderable = table.Finish();

using Panel panel = new(tableRenderable);
panel.Expand = false;
using Renderable composed = panel.Finish();

System.Console.WriteLine(console.Render(reusable));
System.Console.Write(console.Render(composed));
```

Passing a `Renderable` to a table, panel, tree, columns, alignment, or padding
operation borrows it only for that synchronous call. Native code retains an
independent Arc reference, so the original renderable remains reusable and
must still be disposed by its owner. A successful builder `Finish()` consumes
the builder; native failure before transfer leaves it live.

Strings use strict length-delimited UTF-8. Embedded NUL is preserved and an
unpaired UTF-16 surrogate is rejected. Native output is copied to a managed
string and its `RichBuffer` is released in a `finally` block on success or
failure; callers never manage native output memory.

Trees expose stable `NodeId` values, with `NodeId.Root` / `Tree.RootNodeId`
naming the root. IDs contain no pointer and are valid only while their builder
is live. Progress task IDs are unsigned and task ID zero is valid.

`Progress`, `Status`, and `Spinner` render one caller-requested frame. The
binding creates no timer, callback, refresh worker, or managed object retained
by Rust. Applications control frame timing and terminal updates.

Native failures throw `RichException` with a stable `RichErrorCode`:
`NullArgument`, `InvalidUtf8`, `InvalidArgument`, `InvalidState`, `Io`,
`Render`, `NativePanic`, or managed-only `AbiMismatch`. Styles, JSON, themes,
spinner names, task/node IDs, and finite-number rules remain native-authoritative.

## ABI v2 composition and custom rendering

[`examples/demo.rs`](../../examples/demo.rs) is the canonical demo. The .NET
demo constructs the same card through the public API; `--parity` fixes the
console at 120 by 72, forced-terminal truecolor and emits deterministic UTF-8
apart from the two measured durations.

`Table.CreateGrid()` exposes native grid defaults. `SetPadEdge`,
`SetRowStyles`, `Box.None`, and `AddColumnOptions` project native layout
without a managed layout engine. Optional `ColumnOptions` values map to the
ABI presence flags, and a renderable header remains borrowed and reusable.
`Pretty` is a normal `SafeHandle` builder, while `Syntax.SetIndentGuides`
enables native syntax guides.

```csharp
using Table table = Table.CreateGrid();
table.Box = Box.None;
table.SetPadEdge(false);
table.SetRowStyles(["", "dim"]);
table.AddColumnOptions(new ColumnOptions(
    Justification: Justify.Right,
    MinimumWidth: 12,
    Ratio: 1,
    Style: "cyan",
    NoWrap: true));
```

Implement `ICustomRenderer` for synchronous, layout-aware native segments.
`RenderOptions` and `Measurement` are immutable managed values. `SegmentSink`
is valid only on the invoking thread during `Render`; the wrapper tombstones
it on callback exit, so retained or wrong-thread use fails locally. All other
binding calls, including disposal, are locally prohibited inside callbacks.
Managed exceptions are contained by static Cdecl trampolines and rethrown
after the outer `RichConsole.Render` unwinds. The renderer is held by a
`GCHandle` until the last native Arc is released; no callback is dispatched to
a worker or retained by live widgets.

```csharp
sealed class Rule : ICustomRenderer
{
    public void Render(RenderOptions options, SegmentSink sink)
    {
        sink.Push(new string('=', checked((int)options.MaxWidth)), "green");
        sink.Line();
    }

    public Measurement Measure(RenderOptions options) => new(1, options.MaxWidth);
}

using Renderable custom = CustomRenderable.Create(new Rule());
```
