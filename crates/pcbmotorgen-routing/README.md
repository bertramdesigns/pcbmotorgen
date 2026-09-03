# pcbmotorgen-routing

`pcbmotorgen-routing` is the leaf Rust crate that turns a motor-board context
into validated PCB stator routing geometry. It is intentionally independent of
the application's `LinearMotorConfig` and of the physics and KiCad crates, so it
can be developed and tested alone.

**The full API reference is [`docs/API.md`](docs/API.md).** It consolidates the
packaging overview, this README, `SPEC.md`, the field-level handoff contract,
and the plugin authoring guide into one document.

## Scope and ownership

The crate owns the routing-pattern plugin contract, raw geometry model, strict
validator, loaders, the bundled two-layer `infinity-braid` pattern, and the
pole-pitch / phase-band width dimension sidecar.

It does **not** own KiCad layer-name mapping (stays in `pcbmotorgen-export`),
DFM/design rules or interference diagnostics (stays downstream in
`pcbmotorgen-dfm` since kata 0rgs — any routing is allowed in the generator;
DFM is checked later as diagnostics), copper widths supplied by the core, or
the physics/force model (`pcbmotorgen-simulation`).

## Units and axes

- All routing lengths and dimensions are **millimetres**.
- `x` is the travel axis (stator length); `y` is across the board width.
- Copper layer numbers are zero-based indexes into the board stack
  (`0 .. num_layers`).
- `net` labels are ASCII phase names (`"A"`, `"B"`, `"C"`); the KiCad writer
  prefixes `/`.

## The two output levels

1. **Strict plugin output:** `RoutingResult` — `segments`, `curves`, `vias`, and
   optional `pole_regions`, each element carrying its own `layer` and `net`.
   Every bundled pattern, native `cdylib` plugin, and Python runner produces
   exactly this shape. Plugins emit geometry only — never trace widths, via
   sizes, KiCad layer names, or a host-side `RoutingReport` envelope. The wire
   contract is `format_version = 2` (millimetres; version-1 metre payloads are
   rejected).
2. **Application handoff:** `RoutingReport` wraps the validated `RoutingResult`
   with a `dimensions` sidecar (pole pitch, phase-band budget, per-band
   widths). The `generate_coils` IPC response carries the same measurements as
   `routing_dimensions`. The application adapter converts routing millimetres to
   its existing SI/meter frontend DTO at that boundary.

## Design equations

```text
w_s = (N * w_t + (N - 1) * s) / sin(theta)
max(w_s) = tau_p / m - g_phase
```

- `N` = trace count, `w_t` = trace width, `s` = trace spacing, `theta` = angle
  from the travel axis;
- `tau_p` = centre-to-centre North/South pole pitch (not the magnet's physical
  width), `m` = phases, `g_phase` = phase clearance (core minimum spacing rule).

A negative width margin is a DFM diagnostic — never permission to alter the
generated geometry. Worked examples are in [`docs/API.md`](docs/API.md)
(§10.1).

## Commands

Run these from the repository root:

```bash
# Focused routing tests, including the strict validator and dimension equations
cargo test -p pcbmotorgen-routing

# Compile the routing package without running tests
cargo build -p pcbmotorgen-routing

# Check the routing consumers (KiCad/DXF adapter + simulation)
cargo check -p pcbmotorgen-export -p pcbmotorgen-simulation

# Full workspace verification
cargo test --workspace
cargo build --workspace

# Frontend contract/tests for the generate_coils adapter (from app/desktop)
cd app/desktop
pnpm test
pnpm run build
```

Python runner smoke test:

```bash
python3 crates/pcbmotorgen-export/scripts/pattern_runners/example_runner.py --metadata
```

The package has no Python dependency for normal Rust builds. Python is only
needed when a user installs a Python runner.

## Loading a pattern

- **Python runner:** a `.py` script that reads the flattened `RoutingContext`
  JSON on stdin, prints one strict `RoutingResult` JSON on stdout, and
  optionally exposes metadata via `--metadata`. It passes the same validator as
  native plugins.
- **Native plugin:** a `cdylib` exposing the `pcbmotorgen_ROUTING_PLUGIN_API`
  version tag plus `pcbmotorgen_routing_plugin_create` / `_destroy`. Built
  against the same `pcbmotorgen-routing` version as the host; probed and
  validated on upload.

See [`docs/API.md`](docs/API.md) (§12) for the full ABI, the upload/persistence
flow, and the plugin README template.

## Tests and guarantees

`cargo test -p pcbmotorgen-routing` covers NaN/Infinity, bounds, layer,
degenerate-shape, net, and continuity rejection; the exact bottom-up and
top-down phase-band width equations; exact pole-pitch alignment for the infinity
braid; per-layer/per-net dimension reporting; and Python runner parsing and
malformed-output rejection.

The validator is the single gate before geometry is previewed, simulated, or
written. A malformed result is rejected with `index`, `field`, `kind`, and a
human-readable message; it is never repaired in place.

## Extension links

- API reference: [`docs/API.md`](docs/API.md)
- Plugin authoring guide: [`docs/routing-pattern-authoring.md`](docs/routing-pattern-authoring.md)
- Field-level handoff contract: [`docs/routing-pattern-handoff.md`](docs/routing-pattern-handoff.md)
- README authoring guide: [`docs/routing-readme-guide.md`](docs/routing-readme-guide.md)
- Reference Python runner: `crates/pcbmotorgen-export/scripts/pattern_runners/example_runner.py`
- Reference braid algorithm: `.ref/pcbBraid`