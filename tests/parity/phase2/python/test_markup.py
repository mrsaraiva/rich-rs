#!/usr/bin/env python3
"""Parity test for markup module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.markup import _parse, escape, render


def format_token(position, text, tag):
    """Format a parse token for comparison."""
    if text is not None:
        return f"({position}, Text({repr(text)}))"
    elif tag is not None:
        if tag.parameters is not None:
            return f"({position}, Tag({repr(tag.name)}, {repr(tag.parameters)}))"
        else:
            return f"({position}, Tag({repr(tag.name)}, None))"
    return f"({position}, None, None)"


def format_span(span):
    """Format a span for comparison."""
    return f"Span({span.start}, {span.end}, {repr(str(span.style))})"


def main():
    print("=== _parse (tokenizer) ===")

    # Plain text
    tokens = list(_parse("hello world"))
    print(f'_parse("hello world"):')
    for t in tokens:
        print(f"  {format_token(*t)}")

    # Single tag
    tokens = list(_parse("[bold]hello[/bold]"))
    print(f'_parse("[bold]hello[/bold]"):')
    for t in tokens:
        print(f"  {format_token(*t)}")

    # Tag with parameters
    tokens = list(_parse("[link=https://example.com]click[/link]"))
    print(f'_parse("[link=https://example.com]click[/link]"):')
    for t in tokens:
        print(f"  {format_token(*t)}")

    # Escaped bracket
    tokens = list(_parse("\\[not a tag]"))
    print(f'_parse("\\\\[not a tag]"):')
    for t in tokens:
        print(f"  {format_token(*t)}")

    # Mixed content
    tokens = list(_parse("Hello [bold]World[/bold]!"))
    print(f'_parse("Hello [bold]World[/bold]!"):')
    for t in tokens:
        print(f"  {format_token(*t)}")

    print("\n=== escape ===")
    print(f'escape("hello world") -> {repr(escape("hello world"))}')
    print(f'escape("[bold]") -> {repr(escape("[bold]"))}')
    print(f'escape("\\\\[bold]") -> {repr(escape("\\[bold]"))}')
    print(f'escape("[not a tag because 123]") -> {repr(escape("[not a tag because 123]"))}')
    print(f'escape("[red]hello[/red]") -> {repr(escape("[red]hello[/red]"))}')

    print("\n=== render (plain text) ===")

    # Plain text (no markup)
    text = render("hello world")
    print(f'render("hello world").plain -> {repr(text.plain)}')

    # Bold text
    text = render("[bold]hello[/bold]")
    print(f'render("[bold]hello[/bold]").plain -> {repr(text.plain)}')

    # Implicit close
    text = render("[bold]hello[/]")
    print(f'render("[bold]hello[/]").plain -> {repr(text.plain)}')

    # Nested tags
    text = render("[bold][italic]hello[/italic][/bold]")
    print(f'render("[bold][italic]hello[/italic][/bold]").plain -> {repr(text.plain)}')

    # Color
    text = render("[red]hello[/red]")
    print(f'render("[red]hello[/red]").plain -> {repr(text.plain)}')

    # Link
    text = render("[link=https://example.com]click here[/link]")
    print(f'render("[link=https://example.com]click here[/link]").plain -> {repr(text.plain)}')

    # Escaped bracket
    text = render("\\[not bold]")
    print(f'render("\\\\[not bold]").plain -> {repr(text.plain)}')

    # Unclosed tag (applies to end)
    text = render("[bold]hello")
    print(f'render("[bold]hello").plain -> {repr(text.plain)}')

    # Multiple styles
    text = render("[bold red on blue]styled[/]")
    print(f'render("[bold red on blue]styled[/]").plain -> {repr(text.plain)}')

    # Overlapping styles
    text = render("[bold]Hello [italic]World[/italic]![/bold]")
    print(f'render("[bold]Hello [italic]World[/italic]![/bold]").plain -> {repr(text.plain)}')

    print("\n=== render (spans) ===")

    # Bold text spans
    text = render("[bold]hello[/bold]")
    print(f'render("[bold]hello[/bold]").spans:')
    for span in text.spans:
        print(f"  {format_span(span)}")

    # Nested tags spans
    text = render("[bold][italic]hello[/italic][/bold]")
    print(f'render("[bold][italic]hello[/italic][/bold]").spans:')
    for span in sorted(text.spans, key=lambda s: (s.start, s.end)):
        print(f"  {format_span(span)}")

    # Multiple tags
    text = render("[red]Hello[/red] [blue]World[/blue]")
    print(f'render("[red]Hello[/red] [blue]World[/blue]").spans:')
    for span in sorted(text.spans, key=lambda s: (s.start, s.end)):
        print(f"  {format_span(span)}")

    print("\n=== render with emoji ===")

    # Emoji replacement
    text = render(":smile:", emoji=True)
    print(f'render(":smile:", emoji=True).plain -> {repr(text.plain)}')

    # Emoji in styled text
    text = render("[bold]:+1:[/bold]", emoji=True)
    print(f'render("[bold]:+1:[/bold]", emoji=True).plain -> {repr(text.plain)}')

    # No emoji replacement
    text = render(":smile:", emoji=False)
    print(f'render(":smile:", emoji=False).plain -> {repr(text.plain)}')


if __name__ == "__main__":
    main()
