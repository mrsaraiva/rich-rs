using RichRs;
using System.Text;

if (args is ["--invalid-string-probe"])
{
    const string invalid = "\ud800";
    MustReject(() => _ = new Text(invalid));
    MustReject(() => _ = Style.Parse(invalid));
    MustReject(() => _ = new Syntax(invalid, "text"));
    MustReject(() => _ = new Syntax("x", invalid));
    MustReject(() => _ = Syntax.FromPath(invalid));
    MustReject(() => _ = new Markdown(invalid));
    MustReject(() => _ = new Json(invalid));
    MustReject(() => _ = new Tree(invalid));
    MustReject(() => _ = new Status(invalid));
    MustReject(() => _ = new Spinner(invalid));
    System.Console.WriteLine("strict-before-native");
    return;
}

if (args is ["--version-only"])
{
    System.Console.WriteLine(RichRuntime.Version);
    return;
}

using RichConsole console = new();
console.ForceTerminal = false;
console.SetSize(80, 24);

using Text text = new("Reusable rich-rs content");
text.SetStyle("bold cyan");
using Renderable reusable = text.Finish();

using Table table = new();
table.SetTitle(".NET 10 binding");
table.AddColumn(reusable);
table.AddColumn("State");
table.AddRow(["borrowed + reusable", "safe"]);
using Renderable tableRenderable = table.Finish();

using Panel panel = new(tableRenderable);
panel.SetTitle("rich-rs from C#");
panel.Expand = false;
using Renderable composed = panel.Finish();

System.Console.WriteLine(console.Render(reusable));
System.Console.Write(console.Render(composed));

static void MustReject(Action action)
{
    try
    {
        action();
        throw new InvalidOperationException("Invalid UTF-16 was accepted.");
    }
    catch (EncoderFallbackException)
    {
    }
}
