#!/bin/bash
set -euo pipefail

CRATES=(
    "midilab"
    "midilab-io"
    "midilab-sim"
)

DRY_RUN=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --execute)
            DRY_RUN=false
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [--execute]"
            echo ""
            echo "Publish midilab crates to crates.io"
            echo ""
            echo "Options:"
            echo "  --execute  Actually publish (default is dry-run)"
            echo "  -h, --help Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

for crate in "${CRATES[@]}"; do
    echo "Publishing $crate..."
    if [[ "$DRY_RUN" == true ]]; then
        cargo publish --dry-run -p "$crate"
    else
        cargo publish -p "$crate"
    fi
    echo ""
done

if [[ "$DRY_RUN" == true ]]; then
    echo "Dry run complete. Use --execute to publish for real."
else
    echo "All crates published successfully."
fi
