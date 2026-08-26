const std = @import("std");
const abi = @import("abi.zig");

/// Advanced access to the generated C declarations. High-level APIs below do
/// not expose C handle or buffer types.
pub const raw = abi.raw;

pub const Error = error{
    NullArgument,
    InvalidUtf8,
    InvalidArgument,
    InvalidState,
    Io,
    Render,
    NativePanic,
    AbiMismatch,
};

pub const ColorSystem = enum(u32) {
    none = raw.RICH_COLOR_NONE,
    standard = raw.RICH_COLOR_STANDARD,
    eight_bit = raw.RICH_COLOR_EIGHT_BIT,
    truecolor = raw.RICH_COLOR_TRUECOLOR,
    windows = raw.RICH_COLOR_WINDOWS,
};

pub const HorizontalAlign = enum(u32) {
    left = raw.RICH_ALIGN_LEFT,
    center = raw.RICH_ALIGN_CENTER,
    right = raw.RICH_ALIGN_RIGHT,
};

pub const VerticalAlign = enum(u32) {
    top = raw.RICH_VERTICAL_TOP,
    middle = raw.RICH_VERTICAL_MIDDLE,
    bottom = raw.RICH_VERTICAL_BOTTOM,
};

pub const Justify = enum(u32) {
    default = raw.RICH_JUSTIFY_DEFAULT,
    left = raw.RICH_JUSTIFY_LEFT,
    center = raw.RICH_JUSTIFY_CENTER,
    right = raw.RICH_JUSTIFY_RIGHT,
    full = raw.RICH_JUSTIFY_FULL,
};

pub const BoxStyle = enum(u32) {
    rounded = raw.RICH_BOX_ROUNDED,
    heavy = raw.RICH_BOX_HEAVY,
    double = raw.RICH_BOX_DOUBLE,
    ascii = raw.RICH_BOX_ASCII,
    minimal = raw.RICH_BOX_MINIMAL,
    square = raw.RICH_BOX_SQUARE,
    simple = raw.RICH_BOX_SIMPLE,
    heavy_head = raw.RICH_BOX_HEAVY_HEAD,
    heavy_edge = raw.RICH_BOX_HEAVY_EDGE,
    double_edge = raw.RICH_BOX_DOUBLE_EDGE,
    horizontals = raw.RICH_BOX_HORIZONTALS,
    minimal_heavy_head = raw.RICH_BOX_MINIMAL_HEAVY_HEAD,
    simple_heavy = raw.RICH_BOX_SIMPLE_HEAVY,
    markdown = raw.RICH_BOX_MARKDOWN,
};

pub const NodeId = u64;
pub const rootNode: NodeId = raw.RICH_TREE_ROOT_ID;

pub const JsonOptions = struct {
    indent: u32 = 2,
    highlight: bool = true,
    sort_keys: bool = false,
};

/// Translate one stable native result category. Unknown codes are treated as
/// an ABI mismatch instead of being guessed at.
pub fn check(result: u32) Error!void {
    return switch (result) {
        raw.RICH_OK => {},
        raw.RICH_ERROR_NULL_ARGUMENT => error.NullArgument,
        raw.RICH_ERROR_INVALID_UTF8 => error.InvalidUtf8,
        raw.RICH_ERROR_INVALID_ARGUMENT => error.InvalidArgument,
        raw.RICH_ERROR_INVALID_STATE => error.InvalidState,
        raw.RICH_ERROR_IO => error.Io,
        raw.RICH_ERROR_RENDER => error.Render,
        raw.RICH_ERROR_PANIC => error.NativePanic,
        else => error.AbiMismatch,
    };
}

pub fn ensureAbi() Error!void {
    if (raw.rich_ffi_abi_version() != raw.RICH_FFI_ABI_VERSION)
        return error.AbiMismatch;
}

/// Return allocator-owned package-version bytes. Free the returned slice with
/// the same allocator.
pub fn version(allocator: std.mem.Allocator) ![]u8 {
    try ensureAbi();
    var buffer = emptyBuffer();
    const result = raw.rich_ffi_version(&buffer);
    return copyAndFreeBuffer(allocator, result, &buffer);
}

fn emptyBuffer() raw.RichBuffer {
    return .{ .data = null, .len = 0 };
}

fn nativeString(value: []const u8) Error!raw.RichStr {
    const len = std.math.cast(u64, value.len) orelse return error.InvalidArgument;
    return .{
        .data = if (value.len == 0) null else value.ptr,
        .len = len,
    };
}

fn nativeBool(value: bool) u8 {
    return @intFromBool(value);
}

fn copyAndFreeBuffer(
    allocator: std.mem.Allocator,
    result: u32,
    buffer: *raw.RichBuffer,
) ![]u8 {
    defer raw.rich_buffer_free(buffer);
    try check(result);

    const len = std.math.cast(usize, buffer.len) orelse return error.AbiMismatch;
    if (len == 0) {
        if (buffer.data != null) return error.AbiMismatch;
        return allocator.alloc(u8, 0);
    }
    if (buffer.data == null) return error.AbiMismatch;
    return allocator.dupe(u8, buffer.data[0..len]);
}

fn requireHandle(comptime T: type, handle: ?*anyopaque) Error!*T {
    return @ptrCast(handle orelse return error.InvalidState);
}

fn newHandle(comptime T: type, constructor: anytype) Error!*anyopaque {
    try ensureAbi();
    var output: ?*T = null;
    try check(constructor(&output));
    return @ptrCast(output orelse return error.AbiMismatch);
}

fn finishHandle(comptime T: type, slot: *?*anyopaque, finishFn: anytype) Error!Renderable {
    return finishHandleWithCleanup(T, slot, finishFn, releaseNativeRenderable);
}

fn releaseNativeRenderable(renderable: *raw.RichRenderable) void {
    raw.rich_renderable_free(renderable);
}

fn finishHandleWithCleanup(
    comptime T: type,
    slot: *?*anyopaque,
    finishFn: anytype,
    cleanupFn: anytype,
) Error!Renderable {
    const live = slot.* orelse return error.InvalidState;
    var input: ?*T = @ptrCast(live);
    var output: ?*raw.RichRenderable = null;
    const result = finishFn(&input, &output);
    slot.* = if (input) |pointer| @ptrCast(pointer) else null;
    const output_aliases_builder = if (output) |renderable|
        finishOutputAliasesBuilder(T, renderable, input)
    else
        false;

    if (result != raw.RICH_OK) {
        discardFinishOutput(output, output_aliases_builder, cleanupFn);
        try check(result);
        unreachable;
    }

    // Native success is one atomic ownership transfer: the builder must have
    // been consumed and exactly one renderable must have been produced. The
    // returned input is the ownership receipt; once it is NULL, an output at
    // the original address may be legal allocator reuse rather than aliasing.
    if (input != null or output_aliases_builder) {
        discardFinishOutput(output, output_aliases_builder, cleanupFn);
        return error.AbiMismatch;
    }
    return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
}

fn finishOutputAliasesBuilder(
    comptime T: type,
    output: *raw.RichRenderable,
    returned_input: ?*T,
) bool {
    if (returned_input) |builder| {
        if (@intFromPtr(output) == @intFromPtr(builder)) return true;
    }
    return false;
}

fn discardFinishOutput(output: ?*raw.RichRenderable, aliases_builder: bool, cleanupFn: anytype) void {
    if (!aliases_builder) {
        if (output) |renderable| cleanupFn(renderable);
    }
}

fn renderableHandle(value: *const Renderable) Error!*const raw.RichRenderable {
    return @ptrCast(value.handle orelse return error.InvalidState);
}

/// A live owner is non-copyable by contract. Keep it in one `var`, pass it by
/// pointer, and call `deinit` exactly as shown in the README. `deinit` itself
/// is idempotent.
pub const Renderable = struct {
    handle: ?*anyopaque,

    pub fn deinit(self: *Renderable) void {
        if (self.handle) |handle| raw.rich_renderable_free(@ptrCast(handle));
        self.handle = null;
    }
};

pub const Console = struct {
    handle: ?*anyopaque,

    pub fn init() Error!Console {
        return .{ .handle = try newHandle(raw.RichConsole, raw.rich_console_new) };
    }

    pub fn deinit(self: *Console) void {
        if (self.handle) |handle| raw.rich_console_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn setForceTerminal(self: *Console, force: bool) Error!void {
        try check(raw.rich_console_set_force_terminal(
            try requireHandle(raw.RichConsole, self.handle),
            nativeBool(force),
        ));
    }

    pub fn setSize(self: *Console, width_value: u32, height: u32) Error!void {
        try check(raw.rich_console_set_size(
            try requireHandle(raw.RichConsole, self.handle),
            width_value,
            height,
        ));
    }

    pub fn setColorSystem(self: *Console, system: ColorSystem) Error!void {
        try check(raw.rich_console_set_color_system(
            try requireHandle(raw.RichConsole, self.handle),
            @intFromEnum(system),
        ));
    }

    pub fn setMarkupEnabled(self: *Console, enabled: bool) Error!void {
        try check(raw.rich_console_set_markup_enabled(
            try requireHandle(raw.RichConsole, self.handle),
            nativeBool(enabled),
        ));
    }

    pub fn setEmojiEnabled(self: *Console, enabled: bool) Error!void {
        try check(raw.rich_console_set_emoji_enabled(
            try requireHandle(raw.RichConsole, self.handle),
            nativeBool(enabled),
        ));
    }

    pub fn width(self: *const Console) Error!u32 {
        var output: u32 = 0;
        try check(raw.rich_console_width(
            try requireHandle(raw.RichConsole, self.handle),
            &output,
        ));
        return output;
    }

    pub fn renderMarkup(
        self: *Console,
        allocator: std.mem.Allocator,
        markup: []const u8,
    ) ![]u8 {
        var buffer = emptyBuffer();
        const result = raw.rich_console_render_markup(
            try requireHandle(raw.RichConsole, self.handle),
            try nativeString(markup),
            &buffer,
        );
        return copyAndFreeBuffer(allocator, result, &buffer);
    }

    pub fn render(
        self: *Console,
        allocator: std.mem.Allocator,
        renderable: *const Renderable,
    ) ![]u8 {
        var buffer = emptyBuffer();
        const result = raw.rich_console_render(
            try requireHandle(raw.RichConsole, self.handle),
            try renderableHandle(renderable),
            &buffer,
        );
        return copyAndFreeBuffer(allocator, result, &buffer);
    }
};

pub const Text = struct {
    handle: ?*anyopaque,

    pub fn init(text: []const u8) Error!Text {
        try ensureAbi();
        var output: ?*raw.RichText = null;
        try check(raw.rich_text_new(try nativeString(text), &output));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn fromMarkup(console: *Console, markup: []const u8) Error!Text {
        try ensureAbi();
        var output: ?*raw.RichText = null;
        try check(raw.rich_text_new_markup(
            try requireHandle(raw.RichConsole, console.handle),
            try nativeString(markup),
            &output,
        ));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn setStyle(self: *Text, style: []const u8) Error!void {
        try check(raw.rich_text_set_style(
            try requireHandle(raw.RichText, self.handle),
            try nativeString(style),
        ));
    }

    pub fn finish(self: *Text) Error!Renderable {
        return finishHandle(raw.RichText, &self.handle, raw.rich_text_finish);
    }

    pub fn deinit(self: *Text) void {
        if (self.handle) |handle| raw.rich_text_free(@ptrCast(handle));
        self.handle = null;
    }
};

pub const Style = struct {
    handle: ?*anyopaque,

    pub fn parse(style: []const u8) Error!Style {
        try ensureAbi();
        var output: ?*raw.RichStyle = null;
        try check(raw.rich_style_parse(try nativeString(style), &output));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn deinit(self: *Style) void {
        if (self.handle) |handle| raw.rich_style_free(@ptrCast(handle));
        self.handle = null;
    }
};

pub const Table = struct {
    handle: ?*anyopaque,

    pub fn init() Error!Table {
        return .{ .handle = try newHandle(raw.RichTable, raw.rich_table_new) };
    }

    pub fn deinit(self: *Table) void {
        if (self.handle) |handle| raw.rich_table_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn finish(self: *Table) Error!Renderable {
        return finishHandle(raw.RichTable, &self.handle, raw.rich_table_finish);
    }

    pub fn setTitle(self: *Table, title: []const u8) Error!void {
        try check(raw.rich_table_set_title(try requireHandle(raw.RichTable, self.handle), try nativeString(title)));
    }

    pub fn setCaption(self: *Table, caption: []const u8) Error!void {
        try check(raw.rich_table_set_caption(try requireHandle(raw.RichTable, self.handle), try nativeString(caption)));
    }

    pub fn setTitleMarkup(self: *Table, console: *Console, markup: []const u8) Error!void {
        try check(raw.rich_table_set_title_markup(
            try requireHandle(raw.RichTable, self.handle),
            try requireHandle(raw.RichConsole, console.handle),
            try nativeString(markup),
        ));
    }

    pub fn setCaptionMarkup(self: *Table, console: *Console, markup: []const u8) Error!void {
        try check(raw.rich_table_set_caption_markup(
            try requireHandle(raw.RichTable, self.handle),
            try requireHandle(raw.RichConsole, console.handle),
            try nativeString(markup),
        ));
    }

    pub fn setBox(self: *Table, style: BoxStyle) Error!void {
        try check(raw.rich_table_set_box(try requireHandle(raw.RichTable, self.handle), @intFromEnum(style)));
    }

    pub fn setShowHeader(self: *Table, show: bool) Error!void {
        try check(raw.rich_table_set_show_header(try requireHandle(raw.RichTable, self.handle), nativeBool(show)));
    }

    pub fn setShowLines(self: *Table, show: bool) Error!void {
        try check(raw.rich_table_set_show_lines(try requireHandle(raw.RichTable, self.handle), nativeBool(show)));
    }

    pub fn setShowEdge(self: *Table, show: bool) Error!void {
        try check(raw.rich_table_set_show_edge(try requireHandle(raw.RichTable, self.handle), nativeBool(show)));
    }

    pub fn setExpand(self: *Table, expand: bool) Error!void {
        try check(raw.rich_table_set_expand(try requireHandle(raw.RichTable, self.handle), nativeBool(expand)));
    }

    pub fn setPadding(self: *Table, left: u32, right: u32) Error!void {
        try check(raw.rich_table_set_padding(try requireHandle(raw.RichTable, self.handle), left, right));
    }

    pub fn setStyle(self: *Table, style: []const u8) Error!void {
        try check(raw.rich_table_set_style(try requireHandle(raw.RichTable, self.handle), try nativeString(style)));
    }

    pub fn addColumn(self: *Table, header: []const u8) Error!void {
        try check(raw.rich_table_add_column(try requireHandle(raw.RichTable, self.handle), try nativeString(header)));
    }

    pub fn addColumnRenderable(self: *Table, header: *const Renderable) Error!void {
        try check(raw.rich_table_add_column_renderable(
            try requireHandle(raw.RichTable, self.handle),
            try renderableHandle(header),
        ));
    }

    pub fn addRowStrings(
        self: *Table,
        allocator: std.mem.Allocator,
        cells: []const []const u8,
    ) !void {
        const table = try requireHandle(raw.RichTable, self.handle);
        const count = std.math.cast(u32, cells.len) orelse return error.InvalidArgument;
        const native = try allocator.alloc(raw.RichStr, cells.len);
        defer allocator.free(native);
        for (cells, native) |cell, *output| output.* = try nativeString(cell);
        try check(raw.rich_table_add_row_strs(
            table,
            if (native.len == 0) null else native.ptr,
            count,
        ));
    }

    pub fn addRowRenderables(
        self: *Table,
        allocator: std.mem.Allocator,
        cells: []const *const Renderable,
    ) !void {
        const table = try requireHandle(raw.RichTable, self.handle);
        const count = std.math.cast(u32, cells.len) orelse return error.InvalidArgument;
        const native = try allocator.alloc(?*const raw.RichRenderable, cells.len);
        defer allocator.free(native);
        for (cells, native) |cell, *output| output.* = try renderableHandle(cell);
        try check(raw.rich_table_add_row_renderables(
            table,
            if (native.len == 0) null else native.ptr,
            count,
        ));
    }
};

pub const Panel = struct {
    handle: ?*anyopaque,

    pub fn init(content: *const Renderable) Error!Panel {
        try ensureAbi();
        var output: ?*raw.RichPanel = null;
        try check(raw.rich_panel_new(try renderableHandle(content), &output));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn deinit(self: *Panel) void {
        if (self.handle) |handle| raw.rich_panel_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn finish(self: *Panel) Error!Renderable {
        return finishHandle(raw.RichPanel, &self.handle, raw.rich_panel_finish);
    }

    pub fn setTitle(self: *Panel, value: []const u8) Error!void {
        try check(raw.rich_panel_set_title(try requireHandle(raw.RichPanel, self.handle), try nativeString(value)));
    }

    pub fn setSubtitle(self: *Panel, value: []const u8) Error!void {
        try check(raw.rich_panel_set_subtitle(try requireHandle(raw.RichPanel, self.handle), try nativeString(value)));
    }

    pub fn setBox(self: *Panel, value: BoxStyle) Error!void {
        try check(raw.rich_panel_set_box(try requireHandle(raw.RichPanel, self.handle), @intFromEnum(value)));
    }

    pub fn setExpand(self: *Panel, value: bool) Error!void {
        try check(raw.rich_panel_set_expand(try requireHandle(raw.RichPanel, self.handle), nativeBool(value)));
    }

    pub fn setWidth(self: *Panel, value: u32) Error!void {
        try check(raw.rich_panel_set_width(try requireHandle(raw.RichPanel, self.handle), value));
    }

    pub fn setPadding(self: *Panel, top: u32, right: u32, bottom: u32, left: u32) Error!void {
        try check(raw.rich_panel_set_padding(
            try requireHandle(raw.RichPanel, self.handle),
            top,
            right,
            bottom,
            left,
        ));
    }

    pub fn setStyle(self: *Panel, value: []const u8) Error!void {
        try check(raw.rich_panel_set_style(try requireHandle(raw.RichPanel, self.handle), try nativeString(value)));
    }

    pub fn setBorderStyle(self: *Panel, value: []const u8) Error!void {
        try check(raw.rich_panel_set_border_style(try requireHandle(raw.RichPanel, self.handle), try nativeString(value)));
    }
};

pub const Tree = struct {
    handle: ?*anyopaque,

    pub fn init(label: []const u8) Error!Tree {
        try ensureAbi();
        var output: ?*raw.RichTree = null;
        try check(raw.rich_tree_new(try nativeString(label), &output));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn initRenderable(label: *const Renderable) Error!Tree {
        try ensureAbi();
        var output: ?*raw.RichTree = null;
        try check(raw.rich_tree_new_renderable(try renderableHandle(label), &output));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn deinit(self: *Tree) void {
        if (self.handle) |handle| raw.rich_tree_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn finish(self: *Tree) Error!Renderable {
        return finishHandle(raw.RichTree, &self.handle, raw.rich_tree_finish);
    }

    pub fn add(self: *Tree, parent: NodeId, label: []const u8) Error!NodeId {
        var output: NodeId = 0;
        try check(raw.rich_tree_add(
            try requireHandle(raw.RichTree, self.handle),
            parent,
            try nativeString(label),
            &output,
        ));
        return output;
    }

    pub fn addRenderable(self: *Tree, parent: NodeId, label: *const Renderable) Error!NodeId {
        var output: NodeId = 0;
        try check(raw.rich_tree_add_renderable(
            try requireHandle(raw.RichTree, self.handle),
            parent,
            try renderableHandle(label),
            &output,
        ));
        return output;
    }

    pub fn setStyle(self: *Tree, style: []const u8) Error!void {
        try check(raw.rich_tree_set_style(try requireHandle(raw.RichTree, self.handle), try nativeString(style)));
    }

    pub fn setGuideStyle(self: *Tree, style: []const u8) Error!void {
        try check(raw.rich_tree_set_guide_style(try requireHandle(raw.RichTree, self.handle), try nativeString(style)));
    }

    pub fn setHideRoot(self: *Tree, hide: bool) Error!void {
        try check(raw.rich_tree_set_hide_root(try requireHandle(raw.RichTree, self.handle), nativeBool(hide)));
    }
};

pub const Rule = struct {
    handle: ?*anyopaque,

    pub fn init() Error!Rule {
        return .{ .handle = try newHandle(raw.RichRule, raw.rich_rule_new) };
    }

    pub fn deinit(self: *Rule) void {
        if (self.handle) |handle| raw.rich_rule_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn finish(self: *Rule) Error!Renderable {
        return finishHandle(raw.RichRule, &self.handle, raw.rich_rule_finish);
    }

    pub fn setTitle(self: *Rule, title: []const u8) Error!void {
        try check(raw.rich_rule_set_title(try requireHandle(raw.RichRule, self.handle), try nativeString(title)));
    }

    pub fn setCharacters(self: *Rule, characters: []const u8) Error!void {
        try check(raw.rich_rule_set_characters(
            try requireHandle(raw.RichRule, self.handle),
            try nativeString(characters),
        ));
    }

    pub fn setStyle(self: *Rule, style: []const u8) Error!void {
        try check(raw.rich_rule_set_style(try requireHandle(raw.RichRule, self.handle), try nativeString(style)));
    }

    pub fn setAlign(self: *Rule, alignment: HorizontalAlign) Error!void {
        try check(raw.rich_rule_set_align(try requireHandle(raw.RichRule, self.handle), @intFromEnum(alignment)));
    }
};

pub const Columns = struct {
    handle: ?*anyopaque,

    pub fn init() Error!Columns {
        return .{ .handle = try newHandle(raw.RichColumns, raw.rich_columns_new) };
    }

    pub fn deinit(self: *Columns) void {
        if (self.handle) |handle| raw.rich_columns_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn finish(self: *Columns) Error!Renderable {
        return finishHandle(raw.RichColumns, &self.handle, raw.rich_columns_finish);
    }

    pub fn add(self: *Columns, renderable: *const Renderable) Error!void {
        try check(raw.rich_columns_add(
            try requireHandle(raw.RichColumns, self.handle),
            try renderableHandle(renderable),
        ));
    }

    pub fn addString(self: *Columns, text: []const u8) Error!void {
        try check(raw.rich_columns_add_str(
            try requireHandle(raw.RichColumns, self.handle),
            try nativeString(text),
        ));
    }

    pub fn setEqual(self: *Columns, equal: bool) Error!void {
        try check(raw.rich_columns_set_equal(try requireHandle(raw.RichColumns, self.handle), nativeBool(equal)));
    }

    pub fn setExpand(self: *Columns, expand: bool) Error!void {
        try check(raw.rich_columns_set_expand(try requireHandle(raw.RichColumns, self.handle), nativeBool(expand)));
    }

    pub fn setPadding(self: *Columns, vertical: u32, horizontal: u32) Error!void {
        try check(raw.rich_columns_set_padding(
            try requireHandle(raw.RichColumns, self.handle),
            vertical,
            horizontal,
        ));
    }
};

pub const Align = struct {
    handle: ?*anyopaque,

    pub fn init(content: *const Renderable, horizontal: HorizontalAlign) Error!Align {
        try ensureAbi();
        var output: ?*raw.RichAlign = null;
        try check(raw.rich_align_new(
            try renderableHandle(content),
            @intFromEnum(horizontal),
            &output,
        ));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn center(content: *const Renderable) Error!Align {
        try ensureAbi();
        var output: ?*raw.RichAlign = null;
        try check(raw.rich_align_center(try renderableHandle(content), &output));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn deinit(self: *Align) void {
        if (self.handle) |handle| raw.rich_align_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn finish(self: *Align) Error!Renderable {
        return finishHandle(raw.RichAlign, &self.handle, raw.rich_align_finish);
    }

    pub fn setVertical(self: *Align, vertical: VerticalAlign) Error!void {
        try check(raw.rich_align_set_vertical(
            try requireHandle(raw.RichAlign, self.handle),
            @intFromEnum(vertical),
        ));
    }

    pub fn setWidth(self: *Align, width_value: u32) Error!void {
        try check(raw.rich_align_set_width(try requireHandle(raw.RichAlign, self.handle), width_value));
    }
};

pub const Padding = struct {
    handle: ?*anyopaque,

    pub fn init(
        content: *const Renderable,
        top: u32,
        right: u32,
        bottom: u32,
        left: u32,
    ) Error!Padding {
        try ensureAbi();
        var output: ?*raw.RichPadding = null;
        try check(raw.rich_padding_new(
            try renderableHandle(content),
            top,
            right,
            bottom,
            left,
            &output,
        ));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn deinit(self: *Padding) void {
        if (self.handle) |handle| raw.rich_padding_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn finish(self: *Padding) Error!Renderable {
        return finishHandle(raw.RichPadding, &self.handle, raw.rich_padding_finish);
    }

    pub fn setStyle(self: *Padding, style: []const u8) Error!void {
        try check(raw.rich_padding_set_style(
            try requireHandle(raw.RichPadding, self.handle),
            try nativeString(style),
        ));
    }

    pub fn setExpand(self: *Padding, expand: bool) Error!void {
        try check(raw.rich_padding_set_expand(
            try requireHandle(raw.RichPadding, self.handle),
            nativeBool(expand),
        ));
    }
};

pub const Syntax = struct {
    handle: ?*anyopaque,

    pub fn init(code: []const u8, lexer: []const u8) Error!Syntax {
        try ensureAbi();
        var output: ?*raw.RichSyntax = null;
        try check(raw.rich_syntax_new(
            try nativeString(code),
            try nativeString(lexer),
            &output,
        ));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn fromPath(path: []const u8) Error!Syntax {
        try ensureAbi();
        var output: ?*raw.RichSyntax = null;
        try check(raw.rich_syntax_from_path(try nativeString(path), &output));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn deinit(self: *Syntax) void {
        if (self.handle) |handle| raw.rich_syntax_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn finish(self: *Syntax) Error!Renderable {
        return finishHandle(raw.RichSyntax, &self.handle, raw.rich_syntax_finish);
    }

    pub fn setTheme(self: *Syntax, theme: []const u8) Error!void {
        try check(raw.rich_syntax_set_theme(
            try requireHandle(raw.RichSyntax, self.handle),
            try nativeString(theme),
        ));
    }

    pub fn setLineNumbers(self: *Syntax, enabled: bool) Error!void {
        try check(raw.rich_syntax_set_line_numbers(
            try requireHandle(raw.RichSyntax, self.handle),
            nativeBool(enabled),
        ));
    }

    pub fn setWordWrap(self: *Syntax, enabled: bool) Error!void {
        try check(raw.rich_syntax_set_word_wrap(
            try requireHandle(raw.RichSyntax, self.handle),
            nativeBool(enabled),
        ));
    }

    pub fn setLineRange(self: *Syntax, start: ?u32, end: ?u32) Error!void {
        try check(raw.rich_syntax_set_line_range(
            try requireHandle(raw.RichSyntax, self.handle),
            nativeBool(start != null),
            start orelse 0,
            nativeBool(end != null),
            end orelse 0,
        ));
    }
};

pub const Markdown = struct {
    handle: ?*anyopaque,

    pub fn init(source: []const u8) Error!Markdown {
        try ensureAbi();
        var output: ?*raw.RichMarkdown = null;
        try check(raw.rich_markdown_new(try nativeString(source), &output));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn deinit(self: *Markdown) void {
        if (self.handle) |handle| raw.rich_markdown_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn finish(self: *Markdown) Error!Renderable {
        return finishHandle(raw.RichMarkdown, &self.handle, raw.rich_markdown_finish);
    }

    pub fn setCodeTheme(self: *Markdown, theme: []const u8) Error!void {
        try check(raw.rich_markdown_set_code_theme(
            try requireHandle(raw.RichMarkdown, self.handle),
            try nativeString(theme),
        ));
    }

    pub fn setHyperlinks(self: *Markdown, enabled: bool) Error!void {
        try check(raw.rich_markdown_set_hyperlinks(
            try requireHandle(raw.RichMarkdown, self.handle),
            nativeBool(enabled),
        ));
    }

    pub fn setJustify(self: *Markdown, justify: Justify) Error!void {
        try check(raw.rich_markdown_set_justify(
            try requireHandle(raw.RichMarkdown, self.handle),
            @intFromEnum(justify),
        ));
    }
};

pub const Json = struct {
    handle: ?*anyopaque,

    pub fn init(data: []const u8, options: JsonOptions) Error!Json {
        try ensureAbi();
        var output: ?*raw.RichJson = null;
        try check(raw.rich_json_new(
            try nativeString(data),
            options.indent,
            nativeBool(options.highlight),
            nativeBool(options.sort_keys),
            &output,
        ));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn deinit(self: *Json) void {
        if (self.handle) |handle| raw.rich_json_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn finish(self: *Json) Error!Renderable {
        return finishHandle(raw.RichJson, &self.handle, raw.rich_json_finish);
    }
};

pub const Progress = struct {
    handle: ?*anyopaque,

    pub fn init() Error!Progress {
        return .{ .handle = try newHandle(raw.RichProgress, raw.rich_progress_new) };
    }

    pub fn deinit(self: *Progress) void {
        if (self.handle) |handle| raw.rich_progress_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn addTask(self: *Progress, description: []const u8, total: f64) Error!u64 {
        var output: u64 = 0;
        try check(raw.rich_progress_add_task(
            try requireHandle(raw.RichProgress, self.handle),
            try nativeString(description),
            total,
            &output,
        ));
        return output;
    }

    pub fn update(self: *Progress, task_id: u64, completed: f64) Error!void {
        try check(raw.rich_progress_update(
            try requireHandle(raw.RichProgress, self.handle),
            task_id,
            completed,
        ));
    }

    pub fn renderFrame(
        self: *const Progress,
        allocator: std.mem.Allocator,
        console: *Console,
    ) ![]u8 {
        var buffer = emptyBuffer();
        const result = raw.rich_progress_render_frame(
            try requireHandle(raw.RichProgress, self.handle),
            try requireHandle(raw.RichConsole, console.handle),
            &buffer,
        );
        return copyAndFreeBuffer(allocator, result, &buffer);
    }
};

pub const Status = struct {
    handle: ?*anyopaque,

    pub fn init(message: []const u8) Error!Status {
        try ensureAbi();
        var output: ?*raw.RichStatus = null;
        try check(raw.rich_status_new(try nativeString(message), &output));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn deinit(self: *Status) void {
        if (self.handle) |handle| raw.rich_status_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn renderFrame(
        self: *const Status,
        allocator: std.mem.Allocator,
        console: *Console,
    ) ![]u8 {
        var buffer = emptyBuffer();
        const result = raw.rich_status_render_frame(
            try requireHandle(raw.RichStatus, self.handle),
            try requireHandle(raw.RichConsole, console.handle),
            &buffer,
        );
        return copyAndFreeBuffer(allocator, result, &buffer);
    }
};

pub const Spinner = struct {
    handle: ?*anyopaque,

    pub fn init(name: []const u8) Error!Spinner {
        try ensureAbi();
        var output: ?*raw.RichSpinner = null;
        try check(raw.rich_spinner_new(try nativeString(name), &output));
        return .{ .handle = @ptrCast(output orelse return error.AbiMismatch) };
    }

    pub fn deinit(self: *Spinner) void {
        if (self.handle) |handle| raw.rich_spinner_free(@ptrCast(handle));
        self.handle = null;
    }

    pub fn renderFrame(
        self: *const Spinner,
        allocator: std.mem.Allocator,
        console: *Console,
        time_seconds: f64,
    ) ![]u8 {
        var buffer = emptyBuffer();
        const result = raw.rich_spinner_render_frame(
            try requireHandle(raw.RichSpinner, self.handle),
            try requireHandle(raw.RichConsole, console.handle),
            time_seconds,
            &buffer,
        );
        return copyAndFreeBuffer(allocator, result, &buffer);
    }
};

var finish_test_output_storage: usize = 0;
var finish_test_returned_input_storage: u8 = 0;
var finish_test_cleanup_count: usize = 0;

fn finishTestOutput() *raw.RichRenderable {
    return @ptrCast(&finish_test_output_storage);
}

fn recordFinishTestCleanup(renderable: *raw.RichRenderable) void {
    _ = renderable;
    finish_test_cleanup_count += 1;
}

fn malformedFailureKeepsInput(
    input: *?*u8,
    output: *?*raw.RichRenderable,
) u32 {
    std.debug.assert(input.* != null);
    output.* = finishTestOutput();
    return raw.RICH_ERROR_INVALID_ARGUMENT;
}

fn malformedFailureConsumesInput(
    input: *?*u8,
    output: *?*raw.RichRenderable,
) u32 {
    input.* = null;
    output.* = finishTestOutput();
    return raw.RICH_ERROR_INVALID_STATE;
}

fn malformedSuccessKeepsInput(
    input: *?*u8,
    output: *?*raw.RichRenderable,
) u32 {
    std.debug.assert(input.* != null);
    output.* = finishTestOutput();
    return raw.RICH_OK;
}

fn malformedSuccessOmitsOutput(
    input: *?*u8,
    output: *?*raw.RichRenderable,
) u32 {
    input.* = null;
    output.* = null;
    return raw.RICH_OK;
}

fn failureOutputAliasesLiveOriginal(
    input: *?*u8,
    output: *?*raw.RichRenderable,
) u32 {
    output.* = @ptrCast(input.*.?);
    return raw.RICH_ERROR_INVALID_ARGUMENT;
}

fn successOutputAliasesLiveOriginal(
    input: *?*u8,
    output: *?*raw.RichRenderable,
) u32 {
    output.* = @ptrCast(input.*.?);
    return raw.RICH_OK;
}

fn failureOutputAliasesConsumedOriginal(
    input: *?*u8,
    output: *?*raw.RichRenderable,
) u32 {
    const original = input.*.?;
    input.* = null;
    output.* = @ptrCast(original);
    return raw.RICH_ERROR_INVALID_STATE;
}

fn successOutputAliasesConsumedOriginal(
    input: *?*u8,
    output: *?*raw.RichRenderable,
) u32 {
    const original = input.*.?;
    input.* = null;
    output.* = @ptrCast(original);
    return raw.RICH_OK;
}

fn failureOutputAliasesReturnedInput(
    input: *?*u8,
    output: *?*raw.RichRenderable,
) u32 {
    input.* = &finish_test_returned_input_storage;
    output.* = @ptrCast(input.*.?);
    return raw.RICH_ERROR_INVALID_ARGUMENT;
}

fn successOutputAliasesReturnedInput(
    input: *?*u8,
    output: *?*raw.RichRenderable,
) u32 {
    input.* = &finish_test_returned_input_storage;
    output.* = @ptrCast(input.*.?);
    return raw.RICH_OK;
}

test "finish reconciliation rejects malformed transfers and cleans outputs" {
    var input_storage: u8 = 1;

    finish_test_cleanup_count = 0;
    var failure_live_slot: ?*anyopaque = @ptrCast(&input_storage);
    try std.testing.expectError(
        error.InvalidArgument,
        finishHandleWithCleanup(
            u8,
            &failure_live_slot,
            malformedFailureKeepsInput,
            recordFinishTestCleanup,
        ),
    );
    try std.testing.expect(failure_live_slot != null);
    try std.testing.expectEqual(@as(usize, 1), finish_test_cleanup_count);

    finish_test_cleanup_count = 0;
    var failure_consumed_slot: ?*anyopaque = @ptrCast(&input_storage);
    try std.testing.expectError(
        error.InvalidState,
        finishHandleWithCleanup(
            u8,
            &failure_consumed_slot,
            malformedFailureConsumesInput,
            recordFinishTestCleanup,
        ),
    );
    try std.testing.expect(failure_consumed_slot == null);
    try std.testing.expectEqual(@as(usize, 1), finish_test_cleanup_count);

    finish_test_cleanup_count = 0;
    var malformed_success_slot: ?*anyopaque = @ptrCast(&input_storage);
    try std.testing.expectError(
        error.AbiMismatch,
        finishHandleWithCleanup(
            u8,
            &malformed_success_slot,
            malformedSuccessKeepsInput,
            recordFinishTestCleanup,
        ),
    );
    try std.testing.expect(malformed_success_slot != null);
    try std.testing.expectEqual(@as(usize, 1), finish_test_cleanup_count);

    finish_test_cleanup_count = 0;
    var missing_output_slot: ?*anyopaque = @ptrCast(&input_storage);
    try std.testing.expectError(
        error.AbiMismatch,
        finishHandleWithCleanup(
            u8,
            &missing_output_slot,
            malformedSuccessOmitsOutput,
            recordFinishTestCleanup,
        ),
    );
    try std.testing.expect(missing_output_slot == null);
    try std.testing.expectEqual(@as(usize, 0), finish_test_cleanup_count);
}

test "finish reconciliation uses the returned input as the ownership receipt" {
    var original_input_storage: u8 = 1;

    finish_test_cleanup_count = 0;
    var failure_live_slot: ?*anyopaque = @ptrCast(&original_input_storage);
    try std.testing.expectError(
        error.InvalidArgument,
        finishHandleWithCleanup(
            u8,
            &failure_live_slot,
            failureOutputAliasesLiveOriginal,
            recordFinishTestCleanup,
        ),
    );
    try std.testing.expectEqual(@intFromPtr(&original_input_storage), @intFromPtr(failure_live_slot.?));
    try std.testing.expectEqual(@as(usize, 0), finish_test_cleanup_count);

    finish_test_cleanup_count = 0;
    var success_live_slot: ?*anyopaque = @ptrCast(&original_input_storage);
    try std.testing.expectError(
        error.AbiMismatch,
        finishHandleWithCleanup(
            u8,
            &success_live_slot,
            successOutputAliasesLiveOriginal,
            recordFinishTestCleanup,
        ),
    );
    try std.testing.expectEqual(@intFromPtr(&original_input_storage), @intFromPtr(success_live_slot.?));
    try std.testing.expectEqual(@as(usize, 0), finish_test_cleanup_count);

    finish_test_cleanup_count = 0;
    var failure_consumed_slot: ?*anyopaque = @ptrCast(&original_input_storage);
    try std.testing.expectError(
        error.InvalidState,
        finishHandleWithCleanup(
            u8,
            &failure_consumed_slot,
            failureOutputAliasesConsumedOriginal,
            recordFinishTestCleanup,
        ),
    );
    try std.testing.expect(failure_consumed_slot == null);
    try std.testing.expectEqual(@as(usize, 1), finish_test_cleanup_count);

    finish_test_cleanup_count = 0;
    var success_consumed_slot: ?*anyopaque = @ptrCast(&original_input_storage);
    var adopted_same_address = try finishHandleWithCleanup(
        u8,
        &success_consumed_slot,
        successOutputAliasesConsumedOriginal,
        recordFinishTestCleanup,
    );
    try std.testing.expect(success_consumed_slot == null);
    try std.testing.expectEqual(
        @intFromPtr(&original_input_storage),
        @intFromPtr(adopted_same_address.handle.?),
    );
    try std.testing.expectEqual(@as(usize, 0), finish_test_cleanup_count);
    // The seam uses storage rather than a native renderable; relinquish the
    // synthetic handle without calling the real destructor.
    adopted_same_address.handle = null;

    finish_test_cleanup_count = 0;
    var failure_returned_slot: ?*anyopaque = @ptrCast(&original_input_storage);
    try std.testing.expectError(
        error.InvalidArgument,
        finishHandleWithCleanup(
            u8,
            &failure_returned_slot,
            failureOutputAliasesReturnedInput,
            recordFinishTestCleanup,
        ),
    );
    try std.testing.expectEqual(
        @intFromPtr(&finish_test_returned_input_storage),
        @intFromPtr(failure_returned_slot.?),
    );
    try std.testing.expectEqual(@as(usize, 0), finish_test_cleanup_count);

    finish_test_cleanup_count = 0;
    var success_returned_slot: ?*anyopaque = @ptrCast(&original_input_storage);
    try std.testing.expectError(
        error.AbiMismatch,
        finishHandleWithCleanup(
            u8,
            &success_returned_slot,
            successOutputAliasesReturnedInput,
            recordFinishTestCleanup,
        ),
    );
    try std.testing.expectEqual(
        @intFromPtr(&finish_test_returned_input_storage),
        @intFromPtr(success_returned_slot.?),
    );
    try std.testing.expectEqual(@as(usize, 0), finish_test_cleanup_count);
}

test "allocation and result failures still release native output buffers" {
    try ensureAbi();
    var buffer = emptyBuffer();
    const result = raw.rich_ffi_version(&buffer);

    var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{
        .fail_index = 0,
    });
    try std.testing.expectError(
        error.OutOfMemory,
        copyAndFreeBuffer(failing.allocator(), result, &buffer),
    );
    try std.testing.expect(buffer.data == null);
    try std.testing.expectEqual(@as(u64, 0), buffer.len);

    var unknown_result_buffer = emptyBuffer();
    try check(raw.rich_ffi_version(&unknown_result_buffer));
    try std.testing.expectError(
        error.AbiMismatch,
        copyAndFreeBuffer(std.testing.allocator, 0xffff_fffe, &unknown_result_buffer),
    );
    try std.testing.expect(unknown_result_buffer.data == null);
    try std.testing.expectEqual(@as(u64, 0), unknown_result_buffer.len);
}
