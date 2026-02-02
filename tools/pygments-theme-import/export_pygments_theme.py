#!/usr/bin/env python3
"""
Export Pygments themes to JSON format for conversion to tmTheme.

Usage:
    python export_pygments_theme.py <theme_name> [output.json]
    python export_pygments_theme.py --list  # List all available themes

Example:
    python export_pygments_theme.py dracula dracula.json
    python export_pygments_theme.py monokai monokai.json
"""

import sys
import json
from pygments.styles import get_style_by_name, get_all_styles
from pygments.token import Token, STANDARD_TYPES


def token_to_string(token):
    """Convert a token type to its string representation."""
    # Token types have a string representation like 'Token.Comment.Single'
    s = str(token)
    if s.startswith("Token."):
        return s[6:]  # Remove "Token." prefix
    elif s == "Token":
        return "Token"
    return s


def get_all_token_types():
    """Get all standard token types from Pygments."""
    tokens = set()

    def collect_tokens(token):
        tokens.add(token)
        for sub in token.subtypes:
            collect_tokens(sub)

    collect_tokens(Token)
    return tokens


def export_theme(theme_name):
    """Export a Pygments theme to a dictionary."""
    try:
        style_cls = get_style_by_name(theme_name)
    except Exception as e:
        print(f"Error: Could not find theme '{theme_name}': {e}", file=sys.stderr)
        sys.exit(1)

    # Export theme metadata
    theme_data = {
        "name": theme_name,
        "background_color": style_cls.background_color or "#000000",
        "highlight_color": getattr(style_cls, 'highlight_color', None),
        "line_number_color": getattr(style_cls, 'line_number_color', None),
        "line_number_background_color": getattr(style_cls, 'line_number_background_color', None),
        "styles": {}
    }

    # Export styles from the class's styles dict directly
    # This gets all explicitly defined styles, not inherited ones
    for token_type, style_string in style_cls.styles.items():
        if style_string:
            token_name = token_to_string(token_type)
            style_dict = parse_style_string(style_string)
            if style_dict:
                theme_data["styles"][token_name] = style_dict

    return theme_data


def parse_style_string(style_string):
    """Parse a Pygments style string like 'bold #ff0000 bg:#000000'."""
    style_dict = {}
    parts = style_string.split()

    for part in parts:
        if part == 'bold':
            style_dict['bold'] = True
        elif part == 'italic':
            style_dict['italic'] = True
        elif part == 'underline':
            style_dict['underline'] = True
        elif part == 'nobold':
            style_dict['bold'] = False
        elif part == 'noitalic':
            style_dict['italic'] = False
        elif part == 'noundeline':
            style_dict['underline'] = False
        elif part.startswith('bg:'):
            color = part[3:]
            if color.startswith('#'):
                style_dict['bgcolor'] = color
        elif part.startswith('border:'):
            color = part[7:]
            if color.startswith('#'):
                style_dict['border'] = color
        elif part.startswith('#'):
            style_dict['color'] = part

    return style_dict


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    if sys.argv[1] == "--list":
        print("Available Pygments themes:")
        for name in sorted(get_all_styles()):
            print(f"  {name}")
        sys.exit(0)

    theme_name = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else f"{theme_name}.json"

    theme_data = export_theme(theme_name)

    with open(output_file, 'w') as f:
        json.dump(theme_data, f, indent=2)

    print(f"Exported '{theme_name}' to '{output_file}'")
    print(f"  Background: {theme_data['background_color']}")
    print(f"  Styles: {len(theme_data['styles'])} token types")


if __name__ == "__main__":
    main()
