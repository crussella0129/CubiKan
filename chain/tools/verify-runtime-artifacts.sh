#!/usr/bin/env bash
set -euo pipefail

readonly TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_ROOT="$(cd -- "$TOOL_DIR/../.." && pwd -P)"

case "${1:-}" in
    --test-static)
        shift
        [[ $# -le 1 ]] || {
            printf '%s\n' 'usage: verify-runtime-artifacts.sh --test-static [PROJECT_ROOT]' >&2
            exit 2
        }
        exec /usr/bin/python3 -I -S "$TOOL_DIR/verify-runtime-artifacts.py" \
            test-static "${1:-$PROJECT_ROOT}"
        ;;
    --locked)
        shift
        [[ $# -eq 0 ]] || {
            printf '%s\n' 'usage: verify-runtime-artifacts.sh --locked' >&2
            exit 2
        }
        exec /usr/bin/python3 -I -S "$TOOL_DIR/verify-runtime-artifacts.py" \
            locked "$PROJECT_ROOT"
        ;;
    --live)
        shift
        [[ $# -eq 0 ]] || {
            printf '%s\n' 'usage: verify-runtime-artifacts.sh --live' >&2
            exit 2
        }
        exec /usr/bin/python3 -I -S "$TOOL_DIR/verify-runtime-artifacts.py" \
            live "$PROJECT_ROOT"
        ;;
    *)
        printf '%s\n' 'usage: verify-runtime-artifacts.sh --test-static [PROJECT_ROOT] | --locked | --live' >&2
        exit 2
        ;;
esac
