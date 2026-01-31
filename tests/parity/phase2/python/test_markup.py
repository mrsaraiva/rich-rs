#!/usr/bin/env python3
"""Parity test for Markup module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.markup import escape, render


def main():
    print("=== Markup escape() ===")

    result = escape("[bold]")
    print(f'escape("[bold]") -> "{result}"')

    result = escape("\\[bold]")
    print(f'escape("\\\\[bold]") -> "{result}"')

    result = escape("hello world")
    print(f'escape("hello world") -> "{result}"')

    result = escape("[123]")  # Not a tag (starts with digit)
    print(f'escape("[123]") -> "{result}"')

    result = escape("[red]text[/red]")
    print(f'escape("[red]text[/red]") -> "{result}"')

    print("\n=== Markup render() - Basic ===")

    text = render("plain text")
    print(f'render("plain text") -> plain="{text.plain}", spans={len(text.spans)}')

    text = render("[bold]hello[/bold]")
    print(f'render("[bold]hello[/]") -> plain="{text.plain}", spans={len(text.spans)}')

    text = render("[italic]world[/italic]")
    print(f'render("[italic]world[/]") -> plain="{text.plain}", spans={len(text.spans)}')

    text = render("[bold][italic]both[/italic][/bold]")
    print(f'render("[bold][italic]both[/][/]") -> plain="{text.plain}", spans={len(text.spans)}')

    print("\n=== Markup render() - Colors ===")

    text = render("[red]red text[/red]")
    print(f'render("[red]red text[/]") -> plain="{text.plain}", spans={len(text.spans)}')

    text = render("[bold red]styled[/bold red]")
    print(f'render("[bold red]styled[/]") -> plain="{text.plain}", spans={len(text.spans)}')

    text = render("[on blue]bg color[/on blue]")
    print(f'render("[on blue]bg color[/]") -> plain="{text.plain}", spans={len(text.spans)}')

    print("\n=== Markup render() - Implicit close ===")

    text = render("[bold]hello[/]")
    print(f'render("[bold]hello[/]") -> plain="{text.plain}", spans={len(text.spans)}')

    text = render("[red][bold]nested[/][/]")
    print(f'render("[red][bold]nested[/][/]") -> plain="{text.plain}", spans={len(text.spans)}')

    print("\n=== Markup render() - Escaped brackets ===")

    text = render("\\[not a tag]")
    print(f'render("\\\\[not a tag]") -> plain="{text.plain}", spans={len(text.spans)}')

    text = render("before \\[escaped] after")
    print(f'render("before \\\\[escaped] after") -> plain="{text.plain}"')

    print("\n=== Markup render() - Links ===")

    text = render("[link=https://example.com]click[/link]")
    print(f'render("[link=url]click[/link]") -> plain="{text.plain}", spans={len(text.spans)}')

    print("\n=== Markup render() - Emoji ===")

    text = render(":smile:")
    has_emoji = "\U0001f604" in text.plain
    print(f'render(":smile:") -> has_emoji={str(has_emoji).lower()}')

    text = render("[bold]:+1:[/bold]")
    has_emoji = "\U0001f44d" in text.plain
    print(f'render("[bold]:+1:[/]") -> has_emoji={str(has_emoji).lower()}, spans={len(text.spans)}')

    print("\n=== Markup render() - Mixed content ===")

    text = render("Hello [bold]World[/bold]!")
    print(f'render("Hello [bold]World[/] !") -> plain="{text.plain}", spans={len(text.spans)}')

    text = render("[red]A[/red] [blue]B[/blue] [green]C[/green]")
    print(f'render("[red]A[/] [blue]B[/] [green]C[/]") -> plain="{text.plain}", spans={len(text.spans)}')


if __name__ == "__main__":
    main()
