# rich-ffi native binding foundation

`rich-ffi` is the stable C ABI shared by the Zig, C#, Go, and Node projections
of [rich-rs](..). Rendering and allocation stay in Rust; language packages add
only idiomatic ownership and error handling over this boundary. `rich.h` is the
exhaustive ABI and `rich.hpp` is the reference safe C++ projection.

## Artifacts

| Artifact | File | Purpose |
|---|---|---|
| Static library | `librich_ffi.a` / `rich_ffi.lib` | Link the Rust implementation into a consumer |
| Shared library | `librich_ffi.so` / `.dylib` / `rich_ffi.dll` | Runtime-loaded language bindings |
| C header | `include/rich.h` | Generated fixed-width C11 ABI |
| C++ wrapper | `include/rich.hpp` | Move-only RAII owners and exceptions |

Build and deterministically regenerate the committed header with:

```bash
./ffi/regenerate-header.sh
```

Generation is fatal on cbindgen failure. CI runs the command and rejects any
diff in `ffi/include/rich.h`.

## ABI contract

Check the loaded ABI before ordinary calls:

```c
if (rich_ffi_abi_version() != RICH_FFI_ABI_VERSION) {
    /* load a matching native library */
}
```

Every fallible function returns a `RichResult`. `RICH_OK` is success; distinct
codes cover NULL arguments, UTF-8, invalid arguments/state, I/O, rendering, and
caught Rust panics. Output pointers are initialized before other validation.
Destructors are `void`, accept NULL, and contain panics.

`RichStr` borrows UTF-8 bytes for one call. It does not require a terminating
NUL and embedded NUL bytes are legal. `{NULL, 0}` is the empty string.

`RichBuffer` owns bytes allocated by Rust. Copy them, then release them with the
same allocator:

```c
RichBuffer output = {NULL, 0};
RichResult result = rich_console_render_markup(
    console,
    (RichStr){(const uint8_t *)"[bold]ready[/]", 14},
    &output);
if (result == RICH_OK) {
    fwrite(output.data, 1, (size_t)output.len, stdout);
}
rich_buffer_free(&output); /* clears it; safe after every result */
```

Each `*_new` writes an owning opaque handle through `Handle **out`. A typed
builder's `*_finish(&builder, &renderable)` consumes and NULLs the builder only
on success. A validation failure leaves it live, so wrappers do not have to
guess whether to destroy it.

Renderable inputs to Panel, Tree, Columns, Align, Padding, and Table are
borrowed. The container clones Rust's internal shared reference; the caller's
`RichRenderable` stays independently reusable. Tree mutations use stable
`uint64_t` node IDs (`RICH_TREE_ROOT_ID` is zero), never interior pointers.

These checks do not make arbitrary pointers safe. A non-NULL handle must still
be live, produced by this library, accessed according to its ownership rules,
and not concurrently mutated or freed.

## Plain output and live widgets

`rich_console_set_force_terminal(console, 0)` is the daemon/logging contract:
captured output contains neither CSI (`ESC [`) nor OSC (`ESC ]`) escapes.
Terminal mode emits ANSI styling. Output has no automatically appended newline.

Progress, Status, and Spinner are caller-driven frame renderers. The ABI stores
no foreign callback, starts no refresh worker, and never invokes a language
runtime from a Rust background thread. The caller owns timing, cursor movement,
and the refresh loop.

## C and C++ smoke tests

After `cargo build --manifest-path ffi/Cargo.toml`, Linux smoke commands are:

```bash
gcc -std=c11 -Iffi/include ffi/examples/smoke.c \
  ffi/target/debug/librich_ffi.a -lpthread -ldl -lm -o /tmp/rich-c-smoke
/tmp/rich-c-smoke

g++ -std=c++17 -Iffi/include ffi/examples/smoke_hpp.cpp \
  ffi/target/debug/librich_ffi.a -lpthread -ldl -lm -o /tmp/rich-cpp-smoke
/tmp/rich-cpp-smoke
```

CI also runs these two programs under ASan/UBSan on Linux x86_64 and compiles
all family-specific examples with warnings as errors.

## Release matrix

Pushing a `v*` tag builds static and shared libraries on native runners and
publishes `rich-ffi-<version>-<target>.tar.gz` plus `SHA256SUMS`.

| Target | Runner | Libraries |
|---|---|---|
| `x86_64-unknown-linux-gnu` | Ubuntu 22.04 | `.a`, `.so` |
| `aarch64-unknown-linux-gnu` | Ubuntu 22.04 ARM | `.a`, `.so` |
| `aarch64-apple-darwin` | macOS 14 | `.a`, `.dylib` |
| `x86_64-pc-windows-msvc` | Windows 2022 | `.lib`, `.dll`, import library |
| `aarch64-pc-windows-msvc` | Windows 11 ARM | `.lib`, `.dll`, import library |

Each archive contains `include/{rich.h,rich.hpp}` and the target libraries.
Linux static consumers link `pthread`, `dl`, and `m`; Windows consumers also
link the native libraries reported by `cargo rustc -- --print native-static-libs`.
