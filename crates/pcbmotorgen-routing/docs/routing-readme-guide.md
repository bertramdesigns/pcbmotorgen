# README authoring guide for routing packages

This guide is for maintainers adding a routing pattern, changing the routing
handoff, or publishing a new version of `pcbmotorgen-routing`. A good README
should let a new contributor answer four questions without reading the entire
workspace:

1. What does the package produce?
2. What units and coordinate system does it use?
3. How do I run the important checks?
4. Where do I find the plugin and application handoff contracts?

The package README is at
`crates/pcbmotorgen-routing/README.md`. Keep implementation-specific details
there only when they are useful to a plugin author or a consumer. Put the
normative field-by-field contract in
`docs/routing-pattern-handoff.md` instead.

For a separately distributed pattern, put a README beside the runner or Rust
crate as well. It should document the pattern without requiring readers to
reverse-engineer its source.

## Required README sections

### 1. Scope and ownership

Start with a short description of the crate and list what it owns. For this
package that includes the pattern trait, raw geometry model, validator,
loaders, DFM diagnostics, and the dimension sidecar. Explicitly say what it
does **not** own: KiCad layer-name mapping, copper widths supplied by the core,
and the physics model.

### 2. Units and axes

State these conventions near the top:

- all routing lengths are millimetres;
- x is the travel axis;
- y is across the board width; and
- layers are zero-based copper-stack indexes.

Never show an example with bare numbers whose units are ambiguous. A `12`
example should be labelled as `12 mm`.

### 3. Canonical output versus enriched application output

Keep the distinction clear:

- plugins emit strict `RoutingResult` geometry only;
- the host validates that result; and
- `RoutingReport` / the `generate_coils.routing_dimensions` sidecar carries
  pole pitch and phase-band width data calculated from the same context.

Do not tell plugin authors to emit a report envelope from Python. That would
break the strict runner parser. Update the Rust sidecar calculation and this
README when a new design dimension is added.

### 4. Design equations and a worked example

When the package exposes a motor-design quantity, include:

- the equation;
- the meaning and unit of every symbol;
- the direction used for any angle; and
- one numerical example that can be checked by hand.

For phase-band width the README must keep both views together:

```text
w_s = (N * w_t + (N - 1) * s) / sin(theta)
max(w_s) = tau_p / m - g_phase
```

Explain that `tau_p` is the centre-to-centre North/South pole pitch, not the
magnet's physical width, and that a negative width margin is a diagnostic—not
permission to alter the generated geometry.

### 5. Commands

List copy/paste commands for at least:

```bash
cargo test -p pcbmotorgen-routing
cargo build -p pcbmotorgen-routing
cargo check -p pcbmotorgen
cargo test --workspace
```

If a Python runner or native plugin is part of the handoff, include one
metadata command and one generation/build command. Keep command paths relative
to the repository root.

### 6. Extension links

Link the README to:

- the routing-pattern authoring guide;
- the normative handoff document;
- the reference runner; and
- the native plugin example.

This avoids duplicating the complete ABI and JSON schema in several places.

## Updating the README safely

1. Change the Rust model or public API first.
2. Add or update unit tests for the new field/equation.
3. Update `docs/routing-pattern-handoff.md` with the exact wire shape.
4. Update `docs/routing-pattern-authoring.md` if plugin authors are affected.
5. Update this package README with the user-facing explanation and commands.
6. Search for stale names or units:

   ```bash
   rg "coil_topology|single-layer|phase_band_width|pole_pitch|RoutingResult" \
     README.md crates/pcbmotorgen-routing docs app/src app/src-tauri
   ```

7. Run the focused routing tests, then the workspace check/build.

Avoid promising behaviour that is not tested. In particular, do not describe a
pattern as magnet-aligned unless its reported `period_pitch_mm` is exactly the
context's `pole_pitch_mm` within the test tolerance.

## Plugin README template

Copy this outline when publishing a new pattern:

```markdown
# my-pattern

## Purpose
What motor topology does this pattern generate? Is it single- or multi-layer?

## Contract
- Units: millimetres; x = travel; y = board width.
- Entry point: `generate(ctx)` or the Rust `RoutingPattern` implementation.
- Output: strict `RoutingResult` with segments, curves, and vias.
- Layer/net ownership: explain every layer and net emitted.

## Parameters
| key | type | default | range | meaning |
| --- | --- | --- | --- | --- |

List only independent user knobs. Explain which dimensions are derived from
the context rather than copied into parameters.

## Motor dimensions
State how pole pitch, phase-band pitch, phase-band width, trace angle, and any
pattern period are calculated. Include the equations and units.

## Build and install
```bash
# Python runner
python3 my_pattern.py --metadata

# Rust plugin
cargo build --release
```

## Example
Show a context JSON and the important output counts/dimension fields.

## Validation and tests
Document the command that exercises malformed output, bounds, layers, and DFM
limits. State that the host validator rejects output rather than sanitising it.

## Version, author, and license
Keep these values identical to the runner `--metadata` block or Rust trait
accessors.
```

The example should show the **strict geometry JSON**, not a host-side
`RoutingReport` wrapper. Explain any `routing_dimensions` values as consumer
metadata calculated after validation.

## Documentation review checklist

- [ ] Scope and ownership are accurate.
- [ ] Units, axes, layer indexes, and net semantics are explicit.
- [ ] Strict plugin JSON is not confused with `RoutingReport`.
- [ ] Slot-width and pole-pitch equations include symbol definitions.
- [ ] A worked example matches the Rust tests.
- [ ] Focused and workspace commands are copy/pasteable.
- [ ] Rust and Python extension paths are linked.
- [ ] No removed legacy topology or single-layer assumption remains.
