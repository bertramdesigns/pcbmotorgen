"""
plugin_entry.py — KiCad PCB Stator Generator entry point
=========================================================
Run this script from an **external terminal** while KiCad 10 is open with a
PCB document active and the IPC API server enabled.

Prerequisites
-------------
1. KiCad 10 must be running.
2. IPC API server must be enabled:
   ``Preferences > Plugins > Enable IPC API``  (check the box, restart KiCad).
3. A ``.kicad_pcb`` file must be open in the PCB editor.
4. Dependencies must be installed in your **system Python** environment
   (NOT KiCad's bundled Python 3.9, which has no kipy/magpylib/numpy):

   .. code-block:: bash

       cd /path/to/pcbmotorgen
       pip install -e ".[dev]"

Usage
-----
From a terminal while KiCad 10 is open:

.. code-block:: bash

    python plugin_entry.py

The script connects over the IPC Unix socket (``/tmp/kicad/api.sock``),
reads the open board, and will (in later phases) generate and write the
stator coils.

**Do not run this from the KiCad scripting console.** That console uses
KiCad's embedded Python 3.9 (the ``pcbnew`` SWIG API), which has no
``kipy`` module and cannot reach the IPC socket from within itself.

KiCad workflow
--------------
1. Launch KiCad 10.  The IPC socket is created automatically on startup.
2. Enable the IPC API in Preferences if not already set.
3. Open a ``.kicad_pcb`` file in the PCB editor.  For iterating on this
   plugin, keep a blank "scratch" board open with the correct stackup
   pre-configured (the programmatic stackup writer is Phase 5).
4. Run ``python plugin_entry.py`` from the terminal.
5. Changes appear immediately in the PCB editor.  The entire generation is
   placed as a single undo step — use Ctrl+Z to roll it back cleanly.

Pipeline (Phase 1 skeleton)
----------------------------
Currently this entry point:

- Connects to the running KiCad 10 instance via the IPC API.
- Prints board information to confirm the connection.
- Instantiates a default MotorConfig and prints a summary.

Subsequent phases will wire up:
  LayerOptimizer → WaveWindingGenerator → ViaGridGenerator →
  ForceEvaluator → KiCadBoardWriter
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

# Ensure the project root is importable when run directly.
_project_root = Path(__file__).parent
if str(_project_root) not in sys.path:
    sys.path.insert(0, str(_project_root))

from pcbmotorgen.config import MotorConfig
from pcbmotorgen.units import mm, mils_to_m

# ---------------------------------------------------------------------------
# Default configuration — edit here or replace with a GUI dialog in Phase 5.
# ---------------------------------------------------------------------------

DEFAULT_CONFIG = MotorConfig(
    name="flying-fader-v1",
    active_area_length_m=mm(195),
    # 10 mm × 10 mm × 4 mm N44H magnets (width × cross-width × height)
    magnet_dims_m=(mm(10), mm(10), mm(4)),
    magnet_count=10,
    magnet_pitch_m=mm(12),
    magnet_remanence_t=1.35,
    phases=3,
    target_force_n=0.5,
    max_current_a=1.0,
    # JLCPCB "Standard" 4-layer design rules (5 mil / 5 mil)
    min_trace_m=mils_to_m(5),
    min_space_m=mils_to_m(5),
    min_via_drill_m=mm(0.2),
    min_via_annular_ring_m=mm(0.1),
    board_width_m=mm(20),
    air_gap_m=mm(0.5),
    max_layers=12,
    drive_frequency_hz=500.0,
)


def _connect_kicad():
    """Establish IPC API connection to the running KiCad 10 instance.

    Returns
    -------
    tuple[kipy.KiCad, kipy.board.Board]
        Connected KiCad API object and the currently open board.

    Raises
    ------
    ImportError
        If ``kicad-python`` is not installed in the current Python environment.
    RuntimeError
        If KiCad is not running, IPC API is not enabled, or no PCB is open.

    Notes
    -----
    The correct API is ``kicad.get_board()`` — not the non-existent
    ``kicad.get_open_boards()``.  ``get_board()`` raises ``ApiError`` with a
    clear message when no PCB document is currently open.
    """
    try:
        import kipy
        from kipy.errors import ConnectionError as KiCadConnectionError, ApiError
    except ImportError as exc:
        raise ImportError(
            "kicad-python is not installed in your system Python environment.\n"
            "Run:  pip install kicad-python\n"
            "Make sure you are using your system Python, not KiCad's bundled "
            "Python 3.9 (which cannot host kipy)."
        ) from exc

    try:
        kicad = kipy.KiCad()
        # get_board() is the correct one-call entry point (kicad.py).
        # It raises ApiError if no PCB document is open.
        board = kicad.get_board()
    except KiCadConnectionError as exc:
        raise RuntimeError(
            "Cannot reach KiCad IPC API socket.\n"
            "Ensure KiCad 10 is running with Preferences > Plugins > Enable IPC API "
            f"checked.\n  Detail: {exc}"
        ) from exc
    except ApiError as exc:
        raise RuntimeError(
            f"KiCad refused the IPC request (code={exc.code}).\n"
            "Make sure a .kicad_pcb file is open in the PCB editor.\n"
            f"  Detail: {exc}"
        ) from exc

    return kicad, board


def run(config: MotorConfig = DEFAULT_CONFIG) -> None:
    """Execute the stator generation pipeline.

    Parameters
    ----------
    config:
        Motor parameters.  Defaults to :data:`DEFAULT_CONFIG`.  Pass a custom
        :class:`~pcbmotorgen.config.MotorConfig` to override.
    """
    print("\n" + "=" * 60)
    print("  KiCad PCB Stator Generator  —  flying fader")
    print("=" * 60)
    print(config.summary())
    print()

    # ------------------------------------------------------------------
    # Step 1: Connect to KiCad
    # ------------------------------------------------------------------
    print("[1/5] Connecting to KiCad 10 IPC API …")
    try:
        kicad, board = _connect_kicad()
        # board.name is the correct attribute (board.board_filename does NOT exist)
        print(f"      Connected.  Board: {board.name or '(untitled)'}")
        layer_count = board.get_copper_layer_count()
        print(f"      Copper layers in open board: {layer_count}")
    except Exception as exc:
        print(f"      ERROR: {exc}")
        print(
            textwrap.dedent("""\
            Troubleshooting:
              1. KiCad 10 must be running.
              2. Preferences > Plugins > Enable IPC API must be checked.
              3. A .kicad_pcb file must be open in the PCB editor.
              4. Run this script from an external terminal using your system
                 Python — NOT from the KiCad scripting console.
            """)
        )
        return

    # ------------------------------------------------------------------
    # Steps 2-5: Placeholders — implemented in subsequent phases.
    # ------------------------------------------------------------------
    print("[2/5] LayerOptimizer …  (Phase 4 — not yet implemented)")
    print("[3/5] WaveWindingGenerator …  (Phase 2 — not yet implemented)")
    print("[4/5] ForceEvaluator …  (Phase 3 — not yet implemented)")
    print("[5/5] KiCadBoardWriter …  (Phase 5 — not yet implemented)")
    print()
    print("Phase 1 scaffold complete.  Run subsequent phases to generate coils.")
    print("=" * 60 + "\n")


# Allow direct execution from a terminal.
if __name__ == "__main__":
    run()
