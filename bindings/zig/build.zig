const std = @import("std");

const native_archive_option = "rich-ffi-static";
const native_include_option = "rich-ffi-include-dir";

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const include_dir_text = b.option(
        []const u8,
        native_include_option,
        "Directory containing the generated rich.h header",
    ) orelse defaultIncludeDir(b);
    const archive_text = b.option(
        []const u8,
        native_archive_option,
        "Exact target-matching rich-ffi static archive",
    ) orelse defaultArchive(target);

    const include_dir = absoluteFromBuildRoot(b, include_dir_text);
    const archive = absoluteFromBuildRoot(b, archive_text);
    requireFile(b, b.pathJoin(&.{ include_dir, "rich.h" }), native_include_option);
    requireFile(b, archive, native_archive_option);

    const rich_module = b.addModule("rich", .{
        .root_source_file = b.path("src/rich.zig"),
        .target = target,
        .optimize = optimize,
    });
    configureNative(rich_module, include_dir, archive, target);

    const tests_module = b.createModule(.{
        .root_source_file = b.path("tests/rich_test.zig"),
        .target = target,
        .optimize = optimize,
        .imports = &.{.{ .name = "rich", .module = rich_module }},
    });
    const tests = b.addTest(.{
        .name = "rich-zig-tests",
        .root_module = tests_module,
    });
    const run_tests = b.addRunArtifact(tests);
    const test_step = b.step("test", "Build and run the Zig binding tests");
    test_step.dependOn(&run_tests.step);

    const demo_module = b.createModule(.{
        .root_source_file = b.path("examples/demo.zig"),
        .target = target,
        .optimize = optimize,
        .imports = &.{.{ .name = "rich", .module = rich_module }},
    });
    const demo = b.addExecutable(.{
        .name = "rich-zig-demo",
        .root_module = demo_module,
    });
    const run_demo = b.addRunArtifact(demo);
    if (b.args) |args| run_demo.addArgs(args);
    const example_step = b.step("example", "Build and run the Zig binding demo");
    example_step.dependOn(&run_demo.step);

    const module_check = b.addTest(.{
        .name = "rich-zig-module-check",
        .root_module = rich_module,
    });
    const run_module_tests = b.addRunArtifact(module_check);
    test_step.dependOn(&run_module_tests.step);
    const check_step = b.step("check", "Compile the binding and demo without running them");
    check_step.dependOn(&module_check.step);
    check_step.dependOn(&demo.step);
    b.default_step.dependOn(check_step);
}

fn configureNative(
    module: *std.Build.Module,
    include_dir: []const u8,
    archive: []const u8,
    target: std.Build.ResolvedTarget,
) void {
    module.addIncludePath(.{ .cwd_relative = include_dir });
    module.addObjectFile(.{ .cwd_relative = archive });
    module.linkSystemLibrary("c", .{});

    switch (target.result.os.tag) {
        .linux => {
            module.linkSystemLibrary("gcc_s", .{});
            module.linkSystemLibrary("util", .{});
            module.linkSystemLibrary("rt", .{});
            module.linkSystemLibrary("pthread", .{});
            module.linkSystemLibrary("m", .{});
            module.linkSystemLibrary("dl", .{});
        },
        .macos => {
            module.linkSystemLibrary("System", .{});
            module.linkSystemLibrary("m", .{});
            module.linkFramework("CoreFoundation", .{});
            module.linkFramework("Security", .{});
        },
        .windows => {
            module.linkSystemLibrary("advapi32", .{});
            module.linkSystemLibrary("bcrypt", .{});
            module.linkSystemLibrary("cfgmgr32", .{});
            module.linkSystemLibrary("dbghelp", .{});
            module.linkSystemLibrary("gdi32", .{});
            module.linkSystemLibrary("kernel32", .{});
            module.linkSystemLibrary("msimg32", .{});
            module.linkSystemLibrary("ntdll", .{});
            module.linkSystemLibrary("opengl32", .{});
            module.linkSystemLibrary("synchronization", .{});
            module.linkSystemLibrary("user32", .{});
            module.linkSystemLibrary("userenv", .{});
            module.linkSystemLibrary("winspool", .{});
            module.linkSystemLibrary("ws2_32", .{});
        },
        else => {},
    }
}

fn defaultIncludeDir(b: *std.Build) []const u8 {
    const packaged_header = b.pathFromRoot("include/rich.h");
    return if (fileExists(b, packaged_header)) "include" else "../../ffi/include";
}

fn defaultArchive(target: std.Build.ResolvedTarget) []const u8 {
    if (!target.query.isNative()) {
        std.debug.print(
            "rich Zig bindings: a cross-target build requires an explicit native archive\n" ++
                "pass -D{s}=<path> with an archive built for the selected target\n",
            .{native_archive_option},
        );
        std.process.exit(1);
    }
    return if (target.result.os.tag == .windows)
        "../../ffi/target/debug/rich_ffi.lib"
    else
        "../../ffi/target/debug/librich_ffi.a";
}

fn absoluteFromBuildRoot(b: *std.Build, path: []const u8) []const u8 {
    return if (std.fs.path.isAbsolute(path)) path else b.pathFromRoot(path);
}

fn requireFile(b: *std.Build, path: []const u8, option_name: []const u8) void {
    if (!fileExists(b, path)) {
        std.debug.print(
            "rich Zig bindings: required file not found: {s}\n" ++
                "pass -D{s}=<path> with a target-matching native input\n",
            .{ path, option_name },
        );
        std.process.exit(1);
    }
}

fn fileExists(b: *std.Build, path: []const u8) bool {
    std.Io.Dir.cwd().access(b.graph.io, path, .{}) catch return false;
    return true;
}
