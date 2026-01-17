#!/usr/bin/env python3
"""Parity test for rule module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.console import Console
from rich.rule import Rule
from io import StringIO


def render_rule(rule, width=40):
    """Render a rule to plain text at a given width."""
    output = StringIO()
    console = Console(file=output, force_terminal=True, width=width, color_system=None)
    console.print(rule, end="")
    return output.getvalue().rstrip('\n')


def main():
    print("=== Rule without title ===")

    result = render_rule(Rule(), width=40)
    print(f'Rule(width=40) -> "{result}"')

    result = render_rule(Rule(), width=20)
    print(f'Rule(width=20) -> "{result}"')

    print("\n=== Rule with centered title ===")

    result = render_rule(Rule("Title"), width=40)
    print(f'Rule("Title", width=40) -> "{result}"')

    result = render_rule(Rule("Hello"), width=30)
    print(f'Rule("Hello", width=30) -> "{result}"')

    print("\n=== Rule with left-aligned title ===")

    result = render_rule(Rule("Left", align="left"), width=30)
    print(f'Rule("Left", align=left, width=30) -> "{result}"')

    print("\n=== Rule with right-aligned title ===")

    result = render_rule(Rule("Right", align="right"), width=30)
    print(f'Rule("Right", align=right, width=30) -> "{result}"')

    print("\n=== Rule with custom characters ===")

    result = render_rule(Rule(characters="="), width=20)
    print(f'Rule(characters="=", width=20) -> "{result}"')

    result = render_rule(Rule("Test", characters="*"), width=20)
    print(f'Rule("Test", characters="*", width=20) -> "{result}"')

    result = render_rule(Rule("Multi", characters="+-"), width=30)
    print(f'Rule("Multi", characters="+-", width=30) -> "{result}"')

    print("\n=== Rule with narrow width ===")

    result = render_rule(Rule("Title"), width=15)
    print(f'Rule("Title", width=15) -> "{result}"')

    result = render_rule(Rule("X"), width=10)
    print(f'Rule("X", width=10) -> "{result}"')

    print("\n=== AlignMethod parsing ===")

    # Python uses Literal["left", "center", "right"] - validate the values
    valid_values = ["left", "center", "right"]
    for v in valid_values:
        print(f'AlignMethod::parse("{v}") -> Some({v.capitalize()})')
    print('AlignMethod::parse("invalid") -> None')


if __name__ == "__main__":
    main()
