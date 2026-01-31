#!/usr/bin/env python3
"""Parity test for Text module."""

import sys
sys.path.insert(0, "/home/msaraiva/dev/mark/Proj/Libs/rich")

from rich.text import Text, Span
from rich.style import Style


def main():
    print("=== Span Methods ===")

    # Span.split()
    span = Span(5, 15, "bold")
    left, right = span.split(10)
    print(f"Span(5,15).split(10) -> ({left.start},{left.end}), ({right.start},{right.end})" if right else f"Span(5,15).split(10) -> ({left.start},{left.end}), None")

    span = Span(5, 15, "bold")
    left, right = span.split(3)  # Before span
    print(f"Span(5,15).split(3) -> ({left.start},{left.end}), {right}")

    span = Span(5, 15, "bold")
    left, right = span.split(20)  # After span
    print(f"Span(5,15).split(20) -> ({left.start},{left.end}), {right}")

    # Span.move()
    span = Span(5, 10, "bold")
    moved = span.move(3)
    print(f"Span(5,10).move(3) -> ({moved.start},{moved.end})")

    span = Span(5, 10, "bold")
    moved = span.move(-2)
    print(f"Span(5,10).move(-2) -> ({moved.start},{moved.end})")

    # Span.right_crop()
    span = Span(5, 15, "bold")
    cropped = span.right_crop(10)
    print(f"Span(5,15).right_crop(10) -> ({cropped.start},{cropped.end})")

    span = Span(5, 15, "bold")
    cropped = span.right_crop(20)  # Beyond end
    print(f"Span(5,15).right_crop(20) -> ({cropped.start},{cropped.end})")

    # Span.extend()
    span = Span(5, 10, "bold")
    extended = span.extend(5)
    print(f"Span(5,10).extend(5) -> ({extended.start},{extended.end})")

    span = Span(5, 10, "bold")
    extended = span.extend(0)
    print(f"Span(5,10).extend(0) -> ({extended.start},{extended.end})")

    print("\n=== Text.from_markup() ===")

    text = Text.from_markup("[bold]Hello[/bold] World")
    print(f'from_markup("[bold]Hello[/] World") -> plain="{text.plain}", spans={len(text.spans)}')

    text = Text.from_markup("[red]Red[/red] and [blue]Blue[/blue]")
    print(f'from_markup("[red]Red[/] and [blue]Blue[/]") -> plain="{text.plain}", spans={len(text.spans)}')

    text = Text.from_markup("No markup here")
    print(f'from_markup("No markup here") -> plain="{text.plain}", spans={len(text.spans)}')

    text = Text.from_markup(":smile: emoji")
    has_emoji = "\U0001f604" in text.plain
    print(f'from_markup(":smile: emoji") -> has_emoji={str(has_emoji).lower()}')

    print("\n=== Text.assemble() ===")

    text = Text.assemble("Hello ", ("World", "bold"))
    print(f'assemble("Hello ", ("World", "bold")) -> plain="{text.plain}", spans={len(text.spans)}')

    text = Text.assemble(("Red", "red"), " and ", ("Blue", "blue"))
    print(f'assemble(("Red", "red"), " and ", ("Blue", "blue")) -> plain="{text.plain}", spans={len(text.spans)}')

    t1 = Text("Styled", style="italic")
    text = Text.assemble("Prefix ", t1, " Suffix")
    print(f'assemble("Prefix ", Text("Styled", style="italic"), " Suffix") -> plain="{text.plain}", spans={len(text.spans)}')

    print("\n=== Text.stylize() ===")

    text = Text("Hello World")
    text.stylize("bold", 0, 5)
    print(f'stylize("bold", 0, 5) -> spans={len(text.spans)}, first=({text.spans[0].start},{text.spans[0].end})')

    text = Text("Hello World")
    text.stylize("bold", -5)  # Last 5 characters
    print(f'stylize("bold", -5) -> spans={len(text.spans)}, first=({text.spans[0].start},{text.spans[0].end})')

    text = Text("Hello World")
    text.stylize("bold", 0, -6)  # First 5 characters
    print(f'stylize("bold", 0, -6) -> spans={len(text.spans)}, first=({text.spans[0].start},{text.spans[0].end})')

    print("\n=== Text.stylize_before() ===")

    text = Text("Hello World")
    text.stylize("bold")
    text.stylize_before("italic")
    spans_order = [("italic" if "italic" in str(s.style) else "bold") for s in text.spans]
    print(f'stylize("bold") then stylize_before("italic") -> order={spans_order}')

    print("\n=== Text.highlight_regex() ===")

    text = Text("Hello World Hello")
    count = text.highlight_regex(r"Hello", "bold")
    print(f'highlight_regex("Hello") -> count={count}, spans={len(text.spans)}')

    text = Text("test123test456")
    count = text.highlight_regex(r"\d+", "red")
    print(f'highlight_regex(r"\\d+") -> count={count}, spans={len(text.spans)}')

    text = Text("No matches here")
    count = text.highlight_regex(r"\d+", "red")
    print(f'highlight_regex(r"\\d+") on "No matches here" -> count={count}')

    print("\n=== Text.highlight_words() ===")

    text = Text("The quick brown fox")
    count = text.highlight_words(["quick", "fox"], "bold")
    print(f'highlight_words(["quick", "fox"]) -> count={count}, spans={len(text.spans)}')

    text = Text("Hello HELLO hello")
    count = text.highlight_words(["hello"], "bold", case_sensitive=False)
    print(f'highlight_words(["hello"], case_sensitive=False) -> count={count}')

    text = Text("Hello HELLO hello")
    count = text.highlight_words(["hello"], "bold", case_sensitive=True)
    print(f'highlight_words(["hello"], case_sensitive=True) -> count={count}')

    print("\n=== Text.divide() ===")

    text = Text("Hello World!")
    divided = text.divide([5, 6])
    plains = [t.plain for t in divided]
    print(f'divide([5, 6]) -> {plains}')

    text = Text("ABCDEFGHIJ")
    divided = text.divide([2, 5, 8])
    plains = [t.plain for t in divided]
    print(f'divide([2, 5, 8]) -> {plains}')

    # Divide with spans
    text = Text("Hello World")
    text.stylize("bold", 0, 5)
    divided = text.divide([5])
    spans_counts = [len(t.spans) for t in divided]
    print(f'divide([5]) with span(0,5) -> span_counts={spans_counts}')

    # Span crossing boundary
    text = Text("Hello World")
    text.stylize("bold", 3, 8)
    divided = text.divide([5])
    spans_counts = [len(t.spans) for t in divided]
    print(f'divide([5]) with span(3,8) crossing -> span_counts={spans_counts}')


if __name__ == "__main__":
    main()
