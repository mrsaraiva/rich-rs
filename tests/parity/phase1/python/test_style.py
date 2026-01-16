#!/usr/bin/env python3
"""Parity test for style module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.style import Style


def main():
    print("=== Style Parsing ===")

    s = Style.parse("bold")
    print(f'parse("bold") -> bold={s.bold}')

    s = Style.parse("italic")
    print(f'parse("italic") -> italic={s.italic}')

    s = Style.parse("bold italic")
    print(f'parse("bold italic") -> bold={s.bold}, italic={s.italic}')

    s = Style.parse("bold red")
    print(f'parse("bold red") -> bold={s.bold}, color={s.color.name if s.color else None}')

    s = Style.parse("bold red on blue")
    print(f'parse("bold red on blue") -> bold={s.bold}, color={s.color.name if s.color else None}, bgcolor={s.bgcolor.name if s.bgcolor else None}')

    s = Style.parse("underline strike")
    print(f'parse("underline strike") -> underline={s.underline}, strike={s.strike}')

    print("\n=== Style Combination ===")

    s1 = Style.parse("bold")
    s2 = Style.parse("italic")
    combined = s1 + s2
    print(f'bold + italic -> bold={combined.bold}, italic={combined.italic}')

    s1 = Style.parse("bold red")
    s2 = Style.parse("blue")
    combined = s1 + s2
    print(f'(bold red) + blue -> bold={combined.bold}, color={combined.color.name if combined.color else None}')

    print("\n=== ANSI Rendering ===")

    s = Style.parse("bold")
    rendered = s.render("X")
    print(f'Style(bold).render("X") -> {escape_ansi(rendered)}')

    s = Style.parse("italic")
    rendered = s.render("X")
    print(f'Style(italic).render("X") -> {escape_ansi(rendered)}')

    s = Style.parse("bold italic")
    rendered = s.render("X")
    print(f'Style(bold italic).render("X") -> {escape_ansi(rendered)}')

    s = Style.parse("red")
    rendered = s.render("X")
    print(f'Style(red).render("X") -> {escape_ansi(rendered)}')

    s = Style.parse("bold red")
    rendered = s.render("X")
    print(f'Style(bold red).render("X") -> {escape_ansi(rendered)}')

    s = Style.parse("on blue")
    rendered = s.render("X")
    print(f'Style(on blue).render("X") -> {escape_ansi(rendered)}')

    print("\n=== Null Style ===")

    s = Style()
    is_null = s.color is None and s.bgcolor is None and s.bold is None
    print(f'Style() is null -> {str(is_null).lower()}')
    print(f'Style().render("X") -> {escape_ansi(s.render("X"))}')

    s = Style.parse("bold")
    is_null = s.color is None and s.bgcolor is None and s.bold is None
    print(f'Style(bold) is null -> {str(is_null).lower()}')


def escape_ansi(s):
    """Convert ANSI escape sequences to readable format."""
    return '"' + s.replace('\x1b', '\\x1b') + '"'


if __name__ == "__main__":
    main()
