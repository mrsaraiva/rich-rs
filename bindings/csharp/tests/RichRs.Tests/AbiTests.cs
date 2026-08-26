using System.Reflection;
using System.Runtime.InteropServices;

namespace RichRs.Tests;

public sealed class AbiTests
{
    [Fact]
    public void MetadataAndLayoutsMatchNativeContract()
    {
        RichRuntime.EnsureAbi();
        Assert.Equal(2U, RichRuntime.AbiVersion);
        Assert.Equal("1.2.2", RichRuntime.Version);
        Assert.Equal(new Version(1, 2, 2, 0), typeof(RichRuntime).Assembly.GetName().Version);
        Assert.Equal(new JsonOptions(2, true, false), new JsonOptions());
        Assert.Equal(IntPtr.Size + sizeof(ulong), Marshal.SizeOf<NativeRichStr>());
        Assert.Equal(IntPtr.Size + sizeof(ulong), Marshal.SizeOf<NativeRichBuffer>());
        Assert.Equal(0, Marshal.OffsetOf<NativeRichStr>(nameof(NativeRichStr.Data)).ToInt32());
        Assert.Equal(IntPtr.Size, Marshal.OffsetOf<NativeRichStr>(nameof(NativeRichStr.Length)).ToInt32());
        Assert.Equal(0, Marshal.OffsetOf<NativeRichBuffer>(nameof(NativeRichBuffer.Data)).ToInt32());
        Assert.Equal(IntPtr.Size, Marshal.OffsetOf<NativeRichBuffer>(nameof(NativeRichBuffer.Length)).ToInt32());
        Assert.Equal(24, Marshal.SizeOf<NativeRichRenderOptions>());
        Assert.Equal(0, Marshal.OffsetOf<NativeRichRenderOptions>(nameof(NativeRichRenderOptions.MinWidth)).ToInt32());
        Assert.Equal(4, Marshal.OffsetOf<NativeRichRenderOptions>(nameof(NativeRichRenderOptions.MaxWidth)).ToInt32());
        Assert.Equal(8, Marshal.OffsetOf<NativeRichRenderOptions>(nameof(NativeRichRenderOptions.MaxHeight)).ToInt32());
        Assert.Equal(12, Marshal.OffsetOf<NativeRichRenderOptions>(nameof(NativeRichRenderOptions.ColorSystem)).ToInt32());
        Assert.Equal(16, Marshal.OffsetOf<NativeRichRenderOptions>(nameof(NativeRichRenderOptions.IsTerminal)).ToInt32());
        Assert.Equal(8, Marshal.SizeOf<NativeRichMeasurement>());
        Assert.Equal(0, Marshal.OffsetOf<NativeRichMeasurement>(nameof(NativeRichMeasurement.Minimum)).ToInt32());
        Assert.Equal(4, Marshal.OffsetOf<NativeRichMeasurement>(nameof(NativeRichMeasurement.Maximum)).ToInt32());
        Assert.Equal(IntPtr.Size * 3, Marshal.SizeOf<NativeRichCustomRenderableVTable>());
        Assert.Equal(0, Marshal.OffsetOf<NativeRichCustomRenderableVTable>(nameof(NativeRichCustomRenderableVTable.Render)).ToInt32());
        Assert.Equal(IntPtr.Size, Marshal.OffsetOf<NativeRichCustomRenderableVTable>(nameof(NativeRichCustomRenderableVTable.Measure)).ToInt32());
        Assert.Equal(IntPtr.Size * 2, Marshal.OffsetOf<NativeRichCustomRenderableVTable>(nameof(NativeRichCustomRenderableVTable.Drop)).ToInt32());
        Assert.Equal(24 + IntPtr.Size + sizeof(ulong), Marshal.SizeOf<NativeRichTableColumnOptions>());
        Assert.Equal(0, Marshal.OffsetOf<NativeRichTableColumnOptions>(nameof(NativeRichTableColumnOptions.Flags)).ToInt32());
        Assert.Equal(4, Marshal.OffsetOf<NativeRichTableColumnOptions>(nameof(NativeRichTableColumnOptions.Justify)).ToInt32());
        Assert.Equal(8, Marshal.OffsetOf<NativeRichTableColumnOptions>(nameof(NativeRichTableColumnOptions.Vertical)).ToInt32());
        Assert.Equal(12, Marshal.OffsetOf<NativeRichTableColumnOptions>(nameof(NativeRichTableColumnOptions.Width)).ToInt32());
        Assert.Equal(16, Marshal.OffsetOf<NativeRichTableColumnOptions>(nameof(NativeRichTableColumnOptions.MinWidth)).ToInt32());
        Assert.Equal(20, Marshal.OffsetOf<NativeRichTableColumnOptions>(nameof(NativeRichTableColumnOptions.Ratio)).ToInt32());
        Assert.Equal(24, Marshal.OffsetOf<NativeRichTableColumnOptions>(nameof(NativeRichTableColumnOptions.Style)).ToInt32());
        Assert.Equal([1U, 2U, 4U, 8U, 16U, 32U], [NativeColumnFlags.HasJustify,
            NativeColumnFlags.HasVertical, NativeColumnFlags.HasWidth, NativeColumnFlags.HasMinimumWidth,
            NativeColumnFlags.HasRatio, NativeColumnFlags.NoWrap]);
        Assert.Equal(14U, (uint)Box.None);
        Assert.True(IntPtr.Size is 4 or 8);
    }

    [Fact]
    public void EveryKnownAndUnknownResultMapsExactly()
    {
        NativeResult.Check(0);
        Assert.Equal(RichErrorCode.NullArgument, Assert.Throws<RichException>(() => NativeResult.Check(1)).Code);
        Assert.Equal(RichErrorCode.InvalidUtf8, Assert.Throws<RichException>(() => NativeResult.Check(2)).Code);
        Assert.Equal(RichErrorCode.InvalidArgument, Assert.Throws<RichException>(() => NativeResult.Check(3)).Code);
        Assert.Equal(RichErrorCode.InvalidState, Assert.Throws<RichException>(() => NativeResult.Check(4)).Code);
        Assert.Equal(RichErrorCode.Io, Assert.Throws<RichException>(() => NativeResult.Check(5)).Code);
        Assert.Equal(RichErrorCode.Render, Assert.Throws<RichException>(() => NativeResult.Check(6)).Code);
        Assert.Equal(RichErrorCode.NativePanic, Assert.Throws<RichException>(() => NativeResult.Check(7)).Code);
        Assert.Equal(RichErrorCode.AbiMismatch, Assert.Throws<RichException>(() => NativeResult.Check(0xffff_fffe)).Code);
    }

    [Fact]
    public void All125HeaderFunctionsAreProjectedAndExported()
    {
        HashSet<string> header = TestSupport.HeaderSymbols();
        HashSet<string> projected = TestSupport.ProjectedSymbols();
        Assert.Equal(125, header.Count);
        Assert.Equal(header.Order(), projected.Order());

        string nativePath = Environment.GetEnvironmentVariable(NativeLibraryResolver.OverrideVariable)
            ?? throw new InvalidOperationException("Native library override is required for tests.");
        nint library = NativeLibrary.Load(nativePath);
        try
        {
            foreach (string symbol in header)
            {
                Assert.True(NativeLibrary.TryGetExport(library, symbol, out _), $"Missing native export: {symbol}");
            }
        }
        finally
        {
            NativeLibrary.Free(library);
        }
    }

    [Fact]
    public void MissingExplicitLibraryHasActionableDiagnosticInIsolatedProcess()
    {
        string root = TestSupport.RepositoryRoot();
        string demo = Path.Combine(root, "bindings", "csharp", "examples", "RichRs.Demo", "bin", "Release", "net10.0", "RichRs.Demo.dll");
        ProcessResult result = TestSupport.RunDotNet(demo, "--version-only", new Dictionary<string, string?>
        {
            [NativeLibraryResolver.OverrideVariable] = Path.Combine(root, "does-not-exist", "librich_ffi.so"),
        });
        Assert.NotEqual(0, result.ExitCode);
        Assert.Contains(NativeLibraryResolver.OverrideVariable, result.StandardError, StringComparison.Ordinal);
        Assert.Contains("Unable to load", result.StandardError, StringComparison.Ordinal);

        ProcessResult relative = TestSupport.RunDotNet(demo, "--version-only", new Dictionary<string, string?>
        {
            [NativeLibraryResolver.OverrideVariable] = "librich_ffi.so",
        });
        Assert.NotEqual(0, relative.ExitCode);
        Assert.Contains("absolute", relative.StandardError, StringComparison.OrdinalIgnoreCase);

        ProcessResult missingDefault = TestSupport.RunDotNet(demo, "--version-only", new Dictionary<string, string?>
        {
            [NativeLibraryResolver.OverrideVariable] = null,
        });
        Assert.NotEqual(0, missingDefault.ExitCode);
        Assert.Contains(NativeLibraryResolver.OverrideVariable, missingDefault.StandardError, StringComparison.Ordinal);
    }

    [Fact]
    public void ExistingInvalidLibraryAndAbiMismatchHaveIsolatedDiagnostics()
    {
        if (!OperatingSystem.IsLinux())
        {
            return;
        }

        string root = TestSupport.RepositoryRoot();
        string demo = Path.Combine(root, "bindings", "csharp", "examples", "RichRs.Demo", "bin", "Release", "net10.0", "RichRs.Demo.dll");
        string temporary = Path.Combine(Path.GetTempPath(), $"richrs-csharp-loader-{Guid.NewGuid():N}");
        Directory.CreateDirectory(temporary);
        try
        {
            string invalid = Path.Combine(temporary, "invalid.so");
            File.WriteAllText(invalid, "not a shared library");
            ProcessResult invalidResult = TestSupport.RunDotNet(demo, "--version-only", new Dictionary<string, string?>
            {
                [NativeLibraryResolver.OverrideVariable] = invalid,
            });
            Assert.NotEqual(0, invalidResult.ExitCode);
            Assert.Contains("Unable to load", invalidResult.StandardError, StringComparison.Ordinal);
            Assert.Contains(NativeLibraryResolver.OverrideVariable, invalidResult.StandardError, StringComparison.Ordinal);

            string source = Path.Combine(temporary, "abi_mismatch.c");
            string library = Path.Combine(temporary, "librich_ffi_mismatch.so");
            File.WriteAllText(source, "#include <stdint.h>\nuint32_t rich_ffi_abi_version(void) { return 999; }\n");
            ProcessResult compiler = TestSupport.RunProcess("cc", ["-shared", "-fPIC", source, "-o", library],
                new Dictionary<string, string?>());
            Assert.Equal(0, compiler.ExitCode);

            ProcessResult mismatch = TestSupport.RunDotNet(demo, "--version-only", new Dictionary<string, string?>
            {
                [NativeLibraryResolver.OverrideVariable] = library,
            });
            Assert.NotEqual(0, mismatch.ExitCode);
            Assert.Contains("ABI mismatch", mismatch.StandardError, StringComparison.OrdinalIgnoreCase);
            Assert.Contains("999", mismatch.StandardError, StringComparison.Ordinal);
        }
        finally
        {
            Directory.Delete(temporary, true);
        }
    }

    [Fact]
    public void InvalidManagedStringsAreRejectedBeforeNativeLoading()
    {
        string root = TestSupport.RepositoryRoot();
        string demo = Path.Combine(root, "bindings", "csharp", "examples", "RichRs.Demo", "bin", "Release", "net10.0", "RichRs.Demo.dll");
        ProcessResult result = TestSupport.RunDotNet(demo, "--invalid-string-probe", new Dictionary<string, string?>
        {
            [NativeLibraryResolver.OverrideVariable] = null,
        });
        Assert.Equal(0, result.ExitCode);
        Assert.Contains("strict-before-native", result.StandardOutput, StringComparison.Ordinal);
    }
}
