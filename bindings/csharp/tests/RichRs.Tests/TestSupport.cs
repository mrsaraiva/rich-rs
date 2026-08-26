using System.Diagnostics;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.RegularExpressions;

namespace RichRs.Tests;

internal static partial class TestSupport
{
    internal static RichConsole PlainConsole()
    {
        RichConsole console = new()
        {
            ForceTerminal = false,
            ColorSystem = ColorSystem.TrueColor,
            MarkupEnabled = true,
            EmojiEnabled = true,
        };
        console.SetSize(80, 24);
        return console;
    }

    internal static Renderable TextRenderable(string value, string? style = null)
    {
        using Text text = new(value);
        if (style is not null)
        {
            text.SetStyle(style);
        }
        return text.Finish();
    }

    internal static string RepositoryRoot()
    {
        DirectoryInfo? current = new(AppContext.BaseDirectory);
        while (current is not null)
        {
            if (File.Exists(Path.Combine(current.FullName, "ffi", "include", "rich.h")))
            {
                return current.FullName;
            }
            current = current.Parent;
        }
        throw new DirectoryNotFoundException("Could not locate rich-rs repository root.");
    }

    internal static HashSet<string> HeaderSymbols()
    {
        string header = File.ReadAllText(Path.Combine(RepositoryRoot(), "ffi", "include", "rich.h"));
        return SymbolRegex().Matches(header).Select(match => match.Groups[1].Value).ToHashSet(StringComparer.Ordinal);
    }

    internal static HashSet<string> ProjectedSymbols() => typeof(NativeMethods)
        .GetMethods(BindingFlags.Static | BindingFlags.NonPublic)
        .Select(method => method.GetCustomAttribute<LibraryImportAttribute>()?.EntryPoint)
        .Where(name => name is not null && name.StartsWith("rich_", StringComparison.Ordinal))
        .Cast<string>()
        .ToHashSet(StringComparer.Ordinal);

    internal static ProcessResult RunDotNet(string dll, string arguments, IReadOnlyDictionary<string, string?> environment)
    {
        ProcessStartInfo start = new("dotnet", $"\"{dll}\" {arguments}")
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        foreach ((string key, string? value) in environment)
        {
            if (value is null)
            {
                start.Environment.Remove(key);
            }
            else
            {
                start.Environment[key] = value;
            }
        }
        using Process process = Process.Start(start) ?? throw new InvalidOperationException("Failed to start dotnet child process.");
        string stdout = process.StandardOutput.ReadToEnd();
        string stderr = process.StandardError.ReadToEnd();
        process.WaitForExit();
        return new(process.ExitCode, stdout, stderr);
    }

    internal static ProcessResult RunProcess(string executable, IReadOnlyList<string> arguments, IReadOnlyDictionary<string, string?> environment)
    {
        ProcessStartInfo start = new(executable)
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        foreach (string argument in arguments)
        {
            start.ArgumentList.Add(argument);
        }
        foreach ((string key, string? value) in environment)
        {
            if (value is null)
            {
                start.Environment.Remove(key);
            }
            else
            {
                start.Environment[key] = value;
            }
        }
        using Process process = Process.Start(start) ?? throw new InvalidOperationException($"Failed to start {executable} child process.");
        string stdout = process.StandardOutput.ReadToEnd();
        string stderr = process.StandardError.ReadToEnd();
        process.WaitForExit();
        return new(process.ExitCode, stdout, stderr);
    }

    [GeneratedRegex(@"\b(rich_[a-z0-9_]+)\s*\(")]
    private static partial Regex SymbolRegex();
}

internal readonly record struct ProcessResult(int ExitCode, string StandardOutput, string StandardError);
