#!/usr/bin/env bash
set -euo pipefail

readonly TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_ROOT="$(cd -- "$TOOL_DIR/../.." && pwd -P)"

case "${1:-}:${2:-}" in
    --locked:--offline)
        [[ $# -eq 2 ]] || exit 2
        exec /usr/bin/python3 -I -S "$TOOL_DIR/verify-weights.py" locked "$PROJECT_ROOT"
        ;;
    --test-static:)
        exec /usr/bin/python3 -I -S "$TOOL_DIR/verify-weights.py" test-static "$PROJECT_ROOT"
        ;;
    --test-static:*)
        [[ $# -eq 2 ]] || exit 2
        exec /usr/bin/python3 -I -S "$TOOL_DIR/verify-weights.py" test-static "$2"
        ;;
    *)
        printf '%s\n' 'usage: verify-weights.sh --locked --offline | --test-static [PROJECT_ROOT]' >&2
        exit 2
        ;;
esac
