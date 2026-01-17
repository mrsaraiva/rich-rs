#!/usr/bin/env python3
"""Parity test for Console module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.console import Console
from rich.theme import Theme


def main():
    print("=== Console.render_str() ===")

    console = Console(force_terminal=True)

    # Basic markup
    text = console.render_str("[bold]Hello[/bold] World")
    print(f'render_str("[bold]Hello[/] World") -> plain="{text.plain}", spans={len(text.spans)}')

    # Nested markup
    text = console.render_str("[bold][red]Nested[/red][/bold]")
    print(f'render_str("[bold][red]Nested[/]") -> plain="{text.plain}", spans={len(text.spans)}')

    # Emoji replacement
    text = console.render_str(":smile: emoji")
    has_emoji = "\U0001f604" in text.plain
    print(f'render_str(":smile: emoji") -> has_emoji={str(has_emoji).lower()}')

    # Markup disabled
    text = console.render_str("[bold]literal[/bold]", markup=False)
    print(f'render_str(markup=False) -> plain="{text.plain}"')

    # Emoji disabled
    text = console.render_str(":smile: literal", emoji=False)
    has_colon = ":smile:" in text.plain
    print(f'render_str(emoji=False) -> has_colon={str(has_colon).lower()}')

    # Both disabled
    text = console.render_str("[bold]:smile:[/bold]", markup=False, emoji=False)
    print(f'render_str(both=False) -> plain="{text.plain}"')

    print("\n=== Theme.styles ===")

    # Default theme has standard styles
    theme = Theme()

    # Check some default style names exist
    has_bold = "bold" in theme.styles
    has_red = "red" in theme.styles
    has_italic = "italic" in theme.styles
    print(f"default theme has 'bold': {str(has_bold).lower()}")
    print(f"default theme has 'red': {str(has_red).lower()}")
    print(f"default theme has 'italic': {str(has_italic).lower()}")

    # Custom theme
    custom_theme = Theme({"custom.test": "bold magenta"})
    has_custom = "custom.test" in custom_theme.styles
    print(f"custom theme has 'custom.test': {str(has_custom).lower()}")

    # Style inheritance
    inherited_theme = Theme({"myerror": "bold red"}, inherit=True)
    has_bold_inherited = "bold" in inherited_theme.styles
    has_myerror = "myerror" in inherited_theme.styles
    print(f"inherited theme has 'bold': {str(has_bold_inherited).lower()}")
    print(f"inherited theme has 'myerror': {str(has_myerror).lower()}")

    print("\n=== Console.get_style() ===")

    console = Console(force_terminal=True)

    # Get standard styles
    style = console.get_style("bold")
    print(f'get_style("bold") -> bold={str(style.bold).lower()}')

    style = console.get_style("italic")
    print(f'get_style("italic") -> italic={str(style.italic).lower()}')

    style = console.get_style("red")
    is_red = style.color is not None and "red" in str(style.color).lower()
    print(f'get_style("red") -> is_red={str(is_red).lower()}')

    # Parse style string
    style = console.get_style("bold red on blue")
    has_bold = style.bold == True
    has_color = style.color is not None
    has_bgcolor = style.bgcolor is not None
    print(f'get_style("bold red on blue") -> bold={str(has_bold).lower()}, has_color={str(has_color).lower()}, has_bgcolor={str(has_bgcolor).lower()}')

    print("\n=== Console with custom theme ===")

    custom = Theme({"highlight": "bold yellow"})
    console = Console(theme=custom, force_terminal=True)

    style = console.get_style("highlight")
    is_bold = style.bold == True
    is_yellow = style.color is not None and "yellow" in str(style.color).lower()
    print(f'get_style("highlight") -> bold={str(is_bold).lower()}, yellow={str(is_yellow).lower()}')


if __name__ == "__main__":
    main()
