#!/bin/bash
# capture-demos.sh - Capture Python Rich and Rust rich-rs demo outputs for comparison
#
# This script runs both demos with identical terminal settings and saves
# the output to /tmp for side-by-side comparison.
#
# Usage: ./tools/capture-demos.sh
#
# Output files:
#   /tmp/rich-py-demo.txt  - Python Rich demo output
#   /tmp/rich-rs-demo.txt  - Rust rich-rs demo output

set -e

# Terminal dimensions
TERM_COLS=200
TERM_ROWS=72

# Output files
PY_OUTPUT="/tmp/rich-py-demo.txt"
RS_OUTPUT="/tmp/rich-rs-demo.txt"

# Colors for script output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Demo Capture Script ===${NC}"
echo "Terminal size: ${TERM_COLS}x${TERM_ROWS}"
echo ""

# Check for required tools
if ! command -v python &> /dev/null; then
    echo -e "${RED}Error: python not found${NC}"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found${NC}"
    exit 1
fi

# Check if 'script' command is available (preferred method)
USE_SCRIPT=false
if command -v script &> /dev/null; then
    USE_SCRIPT=true
fi

# Function to run command with pseudo-terminal at specific size
run_with_pty() {
    local cmd="$1"
    local output="$2"

    # Build environment string to pass into the script command
    local env_vars="COLUMNS=$TERM_COLS LINES=$TERM_ROWS FORCE_COLOR=1 COLORTERM=truecolor TERM=xterm-256color"

    if $USE_SCRIPT; then
        # Use script to create a pseudo-terminal
        # -q: quiet, -c: command, output file
        # Pass environment variables explicitly inside the command
        script -q -c "export $env_vars; stty cols $TERM_COLS rows $TERM_ROWS 2>/dev/null; $cmd" "$output"

        # Remove the trailing "Script done" line and typescript header if present
        # Also remove carriage returns that script adds
        sed -i 's/\r//g' "$output"
        # Remove first line if it's "Script started"
        sed -i '1{/^Script started/d}' "$output"
        # Remove last line if it's "Script done"
        sed -i '${/^Script done/d}' "$output"
    else
        # Fallback: just run with environment variables (less accurate)
        echo -e "${YELLOW}Warning: 'script' command not found, using fallback method${NC}"
        export $env_vars
        eval "$cmd" > "$output" 2>&1
    fi
}

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo -e "${GREEN}Capturing Python Rich demo...${NC}"
run_with_pty "python -m rich" "$PY_OUTPUT"
echo "  Saved to: $PY_OUTPUT"

echo -e "${GREEN}Capturing Rust rich-rs demo...${NC}"
cd "$PROJECT_DIR"
# Build first to avoid build output in capture
cargo build --example demo --quiet 2>/dev/null || cargo build --example demo
run_with_pty "./target/debug/examples/demo" "$RS_OUTPUT"
echo "  Saved to: $RS_OUTPUT"

echo ""
echo -e "${GREEN}Done!${NC}"
echo ""
echo "Output files:"
echo "  Python: $PY_OUTPUT ($(wc -l < "$PY_OUTPUT") lines, $(wc -c < "$PY_OUTPUT") bytes)"
echo "  Rust:   $RS_OUTPUT ($(wc -l < "$RS_OUTPUT") lines, $(wc -c < "$RS_OUTPUT") bytes)"
echo ""
echo "Compare with:"
echo "  diff $PY_OUTPUT $RS_OUTPUT"
echo "  vimdiff $PY_OUTPUT $RS_OUTPUT"
echo ""
echo "View specific sections:"
echo "  grep -A15 'Markdown' $PY_OUTPUT"
echo "  grep -A15 'Markdown' $RS_OUTPUT"
