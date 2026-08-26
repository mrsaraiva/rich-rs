//! Low-level import of the generated rich-ffi C ABI.
//!
//! Most consumers should import `rich.zig`; this module is intentionally a
//! thin, advanced escape hatch whose declarations come only from `rich.h`.

pub const raw = @cImport({
    @cInclude("rich.h");
});

comptime {
    if (@sizeOf(raw.RichResult) != @sizeOf(u32))
        @compileError("rich-ffi RichResult must be exactly 32 bits");
    if (@sizeOf(raw.RichStr) != 16 or @alignOf(raw.RichStr) != @alignOf(u64))
        @compileError("rich-ffi RichStr layout does not match the ABI");
    if (@sizeOf(raw.RichBuffer) != 16 or @alignOf(raw.RichBuffer) != @alignOf(u64))
        @compileError("rich-ffi RichBuffer layout does not match the ABI");
    if (@offsetOf(raw.RichStr, "data") != 0 or @offsetOf(raw.RichStr, "len") != 8)
        @compileError("rich-ffi RichStr field offsets do not match the ABI");
    if (@offsetOf(raw.RichBuffer, "data") != 0 or @offsetOf(raw.RichBuffer, "len") != 8)
        @compileError("rich-ffi RichBuffer field offsets do not match the ABI");
    if (raw.RICH_FFI_ABI_VERSION != 1)
        @compileError("rich-ffi header has an unsupported ABI version");
}
