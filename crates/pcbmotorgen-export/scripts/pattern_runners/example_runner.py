#!/usr/bin/env python3
"""Reference Python routing-pattern runner for pcbmotorgen-routing.

This is a FULLY VALID minimal runner that demonstrates the complete plugin
contract (see docs/routing-pattern-authoring.md):

  1. Default mode: read the flattened RoutingContext JSON from stdin, print
     exactly ONE strict RoutingResult JSON to stdout.

  2. --metadata mode: print a PluginMetadata JSON block (name/author/version/
     description/parameters) and exit 0. Optional — if omitted the app falls
     back to the file stem as the id with blank author/version.

Run the two modes:
    python3 scripts/pattern_runners/example_runner.py < <(echo '{"phases":3,"active_area_length_mm":600.0,"board_width_mm":20.0}')
    python3 scripts/pattern_runners/example_runner.py --metadata
"""
import json
import sys

PLUGIN_METADATA = {
    "id": "example-runner",
    "display_name": "Example Generator (Python)",
    "author": "pcbmotorgen example author",
    "version": "1.0.0",
    "description": "Minimal, valid Python routing pattern: one vertical "
                   "conductor per phase on layer 0.",
    "parameters": [
        {
            "key": "conductor_offset",
            "label": "Conductor offset",
            "description": "Extra x offset for the first conductor [mm].",
            "param_type": "float",
            "default": 0.0,
            "min": 0.0,
            "max": 50.0,
            "step": 1.0,
        }
    ],
}


def generate(ctx: dict) -> dict:
    """Produce a minimal but valid, bounds-conforming RoutingResult.

    This is deliberately trivial so it can serve as a template / smoke test for
    the load path. See pcbmotorgen_routing::patterns::infinity for the real
    braid. All output is raw geometry only — no widths, no via sizes.
    """
    phases = max(int(ctx.get("phases", 3)), 1)
    length = float(ctx.get("active_area_length_mm", 600.0))
    width = float(ctx.get("board_width_mm", 20.0))
    offset = float(ctx.get("params", {}).get("conductor_offset", 0.0))
    segments = [
        {
            "start": {"x": offset + length * i / phases, "y": 0.0},
            "end": {"x": offset + length * i / phases, "y": width},
            "layer": 0,
            "net": chr(ord("A") + i),
            "is_active": True,
        }
        for i in range(phases)
    ]
    return {"segments": segments, "curves": [], "vias": []}


def main() -> None:
    if "--metadata" in sys.argv:
        json.dump(PLUGIN_METADATA, sys.stdout)
        sys.stdout.write("\n")
        return
    ctx = json.load(sys.stdin)
    result = generate(ctx)
    json.dump(result, sys.stdout)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
