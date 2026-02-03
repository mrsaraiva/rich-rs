#!/usr/bin/env python3

from __future__ import annotations

from rich.console import Console
from rich.progress import BarColumn, Progress, TaskProgressColumn, TextColumn, TimeRemainingColumn
from rich.segment import Segment

from test_ansi import hex_utf8, style_repr


def dump_segments(label: str, segments) -> None:
    print(f"CASE|{label}")
    simplified = list(Segment.simplify(segments))
    print(f"COUNT|{len(simplified)}")
    for seg in simplified:
        control = getattr(seg, "control", None)
        if control is not None:
            print(f"CTL|{control}")
            continue
        text = getattr(seg, "text", "")
        style = getattr(seg, "style", None)
        print(f"SEG|{hex_utf8(text)}|{style_repr(style)}")


def main() -> None:
    console = Console(width=80, force_terminal=False, color_system=None)

    progress = Progress(
        TextColumn("[progress.description]{task.description}"),
        BarColumn(),
        TaskProgressColumn(show_speed=False),
        TimeRemainingColumn(elapsed_when_finished=False),
        auto_refresh=False,
        refresh_per_second=10,
        disable=False,
        expand=False,
    )

    progress.add_task("Download", total=100, completed=25, start=True)
    progress.add_task("Process", total=100, completed=90, start=True)

    renderable = progress.get_renderable()
    segments = list(console.render(renderable))
    dump_segments("progress_default_columns", segments)


if __name__ == "__main__":
    main()

