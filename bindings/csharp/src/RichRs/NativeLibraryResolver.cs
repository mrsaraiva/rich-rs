using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace RichRs;

internal static class NativeLibraryResolver
{
    internal const string LogicalName = "rich_ffi";
    internal const string OverrideVariable = "RICH_RS_NATIVE_LIBRARY";

    [ModuleInitializer]
    internal static void Install()
    {
        NativeLibrary.SetDllImportResolver(typeof(NativeLibraryResolver).Assembly, Resolve);
    }

    private static nint Resolve(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (!string.Equals(libraryName, LogicalName, StringComparison.Ordinal))
        {
            return 0;
        }

        string? explicitPath = Environment.GetEnvironmentVariable(OverrideVariable);
        if (!string.IsNullOrWhiteSpace(explicitPath))
        {
            if (!Path.IsPathFullyQualified(explicitPath))
            {
                throw Missing($"{OverrideVariable} must be an absolute native-library path; got '{explicitPath}'.");
            }

            if (NativeLibrary.TryLoad(explicitPath, out nint explicitHandle))
            {
                return explicitHandle;
            }

            throw Missing($"Unable to load rich-rs native library from {OverrideVariable}='{explicitPath}'.");
        }

        string platformName = OperatingSystem.IsWindows()
            ? "rich_ffi.dll"
            : OperatingSystem.IsMacOS() ? "librich_ffi.dylib" : "librich_ffi.so";
        if (NativeLibrary.TryLoad(platformName, assembly, searchPath, out nint handle))
        {
            return handle;
        }

        throw Missing($"Unable to load '{platformName}'. Set {OverrideVariable} to the absolute path of the target-matching rich-ffi shared library.");
    }

    private static DllNotFoundException Missing(string message) => new(message);
}
