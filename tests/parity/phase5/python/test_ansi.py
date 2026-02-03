#!/usr/bin/env python3

from __future__ import annotations

from rich.text import Text


def hex_utf8(s: str) -> str:
    return s.encode("utf-8").hex()


def color_repr(color) -> str:
    if color is None:
        return "-"
    triplet = getattr(color, "triplet", None)
    if triplet is not None:
        r = getattr(triplet, "red", 0)
        g = getattr(triplet, "green", 0)
        b = getattr(triplet, "blue", 0)
        return f"rgb({r},{g},{b})"
    number = getattr(color, "number", None)
    if number is not None:
        return f"n{number}"
    name = getattr(color, "name", None)
    return f"name({name})"


def style_repr(style) -> str:
    if style is None:
        return "-"
    parts = []
    fg = color_repr(getattr(style, "color", None))
    bg = color_repr(getattr(style, "bgcolor", None))
    parts.append(f"fg={fg}")
    parts.append(f"bg={bg}")
    flags = []
    for flag in ("bold", "dim", "italic", "underline", "blink", "reverse", "strike"):
        if getattr(style, flag, False):
            flags.append(flag)
    parts.append("attrs=" + (",".join(flags) if flags else "-"))
    return ";".join(parts)


def dump_text(label: str, text: Text) -> None:
    print(f"CASE|{label}")
    print(f"TEXT|{hex_utf8(text.plain)}")
    base = getattr(text, "style", None)
    print(f"BASE|{style_repr(base)}")
    spans = list(getattr(text, "spans", []))
    for span in spans:
        start = getattr(span, "start", 0)
        end = getattr(span, "end", 0)
        style = getattr(span, "style", None)
        print(f"SPAN|{start}|{end}|{style_repr(style)}")


def main() -> None:
    dump_text("bold_then_reset", Text.from_ansi("\x1b[1mBold\x1b[0m Normal"))
    dump_text("truecolor_fg", Text.from_ansi("\x1b[38;2;255;0;0mRed\x1b[0m"))
    dump_text("persist_across_lines", Text.from_ansi("\x1b[31mred\nstill"))
    dump_text("carriage_return", Text.from_ansi("abc\rdef"))


if __name__ == "__main__":
    main()

