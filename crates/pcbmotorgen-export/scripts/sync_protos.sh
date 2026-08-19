#!/usr/bin/env bash
# Re-sync KiCad 10 IPC .proto files from the kicad-python reference repo into
# the pcbmotorgen-export crate's vendored `proto/` directory.
#
# Usage:
#   ./scripts/sync_protos.sh                # uses the default reference path
#   KICAD_PROTO_REF=/path/to/proto ./scripts/sync_protos.sh
#
set -euo pipefail

DEFAULT_REF="/Users/dylanbertram/.local/share/opencode/repos/gitlab.com/kicad/code/kicad-python/kicad/api/proto"
REF="${KICAD_PROTO_REF:-$DEFAULT_REF}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DST="$SCRIPT_DIR/../proto"

if [ ! -d "$REF" ]; then
    echo "error: reference proto directory not found:" >&2
    echo "  $REF" >&2
    echo "Set KICAD_PROTO_REF to point at kicad-python/kicad/api/proto." >&2
    exit 1
fi

echo "Syncing KiCad .proto files"
echo "  from: $REF"
echo "  to:   $DST"

rm -rf "$DST"
mkdir -p "$DST"
cp -R "$REF/." "$DST/"

PROTO_COUNT=$(find "$DST" -name '*.proto' -type f | wc -l | tr -d ' ')
echo "Synced $PROTO_COUNT .proto files."

echo "Done. Run 'cargo build -p pcbmotorgen-export' to regenerate bindings."
