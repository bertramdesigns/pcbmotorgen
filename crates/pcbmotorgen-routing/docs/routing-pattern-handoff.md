# Routing Handoff Contract — core ↔ routing package

**Status:** Active (contract `format_version = 2`)
**Owner:** `crates/pcbmotorgen-routing` (model, validator, loaders)
**Supersedes:** informal conventions in ADR-0009; this document is the canonical
field-level specification.

## 1. Purpose and scope

This document specifies the **fixed, versioned handoff format** between the
application core (physics/config layer, `app/src-tauri`) and the routing-pattern
package (`pcbmotorgen-routing`). Every routing pattern — bundled, Rust `cdylib`
crate plugin, or Python runner — produces its geometry through this one
interface. The format is the single source of truth consumed by:

- the strict-shape **Validator** (rejection on upload, never sanitised),
- the `PhaseCoil` presentation adapter (`generate/adapters.rs`),
- the **KiCad writer** and **DXF writer**,
- the **force/simulation** model,
- the **`generate_coils` IPC DTO** (`CoilPathIpc`) and the frontend coil preview.

In and out of scope:

| In scope                                        | Out of scope                         |
| ----------------------------------------------- | ------------------------------------ |
| `RoutingContext` (pattern input)                | Any external interchange format (e.g. |
| `RoutingResult` (pattern output)                | circuit-json) — explicitly excluded  |
| JSON encoding for Python runners                | PCB file formats (KiCad/DXF/Gerber)   |
| Version policy for the contract                 | The UI renderer's data model         |

## 2. Coordinate system and unit conventions

| Quantity       | Convention                                                   |
| -------------- | ------------------------------------------------------------ |
| Units          | **millimetres** (mm) for all lengths; base unit throughout the routing wire format |
| X axis         | travel axis (stator length)                                   |
| Y axis         | across board width (perpendicular to travel)                  |
| Z axis         | NOT part of the wire format; layer index (`layer`) encodes depth |
| `layer`        | index into the copper stack `[0, num_layers)` — the **pattern owns layer semantics** |
| `net`          | phase/net label, e.g. `"A"`, `"B"`, `"C"`; ASCII, non-empty; the KiCad writer prefixes `/` |

## 3. Input contract: `RoutingContext`

Flat snapshot handed to every pattern at generate-time. Patterns must not depend
on the concrete core `LinearMotorConfig` — this snapshot is the entire world.

```json
{
  "active_area_length_mm": 195.0,
  "board_width_mm": 20.0,
  "num_layers": 4,
  "phases": 3,
  "min_trace_mm": 0.127,
  "min_space_mm": 0.127,
  "padding_mm": 30.0,
  "expects_continuous": false,
  "params": { "num_strands": 5 },
  "magnet_pitch_mm": 12.0,
  "magnet_array_span_mm": 120.0
}
```

| Field                  | Type                | Meaning                                              |
| ---------------------- | ------------------- | ---------------------------------------------------- |
| `active_area_length_mm` | `f64`               | Copper active-area length along travel [mm].         |
| `board_width_mm`        | `f64`               | PCB dimension perpendicular to travel [mm].          |
| `num_layers`           | `u32`               | Copper layers in the stack.                          |
| `phases`               | `u32`               | Electrical phase count.                              |
| `min_trace_mm`         | `f64`               | Minimum manufacturable trace width [mm].             |
| `min_space_mm`         | `f64`               | Minimum trace-to-trace clearance [mm].               |
| `padding_mm`           | `f64`               | Extra PCB length per end for routing [mm].           |
| `expects_continuous`   | `bool`              | Whether the validator enforces end→start continuity per (layer, net). |
| `params`               | `{string → number}` | Pattern-specific user-editable parameters.           |
| `magnet_pitch_mm`      | `f64?`              | Pole pitch, when the mover layout is known [mm].     |
| `magnet_array_span_mm` | `f64?`              | Mover magnet-array span, when known [mm]. The legacy `coil_span_mm` key is still accepted as a serde alias. |

## 4. Output contract: `RoutingResult`

The canonical geometry document. Every element carries its own `layer` and
`net`; the pattern owns layer semantics.

```json
{
  "format_version": 2,
  "segments": [
    {
      "start": { "x": 0.0, "y": 0.0 },
       "end": { "x": 0.0, "y": 20.0 },
      "layer": 0,
      "net": "A",
      "is_active": true
    }
  ],
  "curves": [
    {
       "start": { "x": 1.0, "y": 20.0 },
       "mid": { "x": 1.6, "y": 20.8 },
       "end": { "x": 2.2, "y": 20.0 },
      "layer": 0,
      "net": "A",
      "is_active": false
    }
  ],
  "vias": [
    {
      "position": { "x": 1.0, "y": 10.0 },
      "from_layer": 0,
      "to_layer": 1,
      "net": "A"
    }
  ]
}
```

| Field            | Type      | Meaning                                                          |
| ---------------- | --------- | ---------------------------------------------------------------- |
| `format_version` | `u32`     | Contract version. Absent ⇒ `2` (see §5).                         |
| `segments[]`     | segment   | Straight traces: `start`/`end` points `{x, y}`, `layer`, `net`, `is_active`. |
| `curves[]`       | curve     | Rounded corners / arcs — quadratic Bézier `start → mid → end`. `mid` is the control point on the arc (matches KiCad `(arc start mid end)`). |
| `vias[]`         | via       | Inter-layer connections: `position`, `from_layer`, `to_layer`, `net`. |

`is_active` distinguishes **force-producing conductors** (`true`) from
**end-turn connectors** (`false`).

### 4.1 Enriched application handoff: `RoutingReport`

The strict plugin payload above remains geometry-only. After generation and
validation, the host may return a [`RoutingReport`](../crates/pcbmotorgen-routing/src/report.rs)
whose `result` is that exact `RoutingResult` plus a `dimensions` sidecar. The
`generate_coils` IPC response exposes the same measurements as
`routing_dimensions`; the Tauri adapter converts those values back to its
existing SI/meter frontend DTO names for compatibility.

The geometry below is abbreviated to one element; a real report contains the
complete validated pattern output.

```json
{
  "result": {
    "format_version": 2,
    "segments": [
      {
        "start": { "x": 0.0, "y": 0.0 },
        "end": { "x": 0.0, "y": 20.0 },
        "layer": 0,
        "net": "A",
        "is_active": true
      }
    ],
    "curves": [],
    "vias": [],
    "pole_regions": [
      {
        "phase": "A",
        "pole_index": 0,
        "start": { "x": 0.0, "y": 10.0 },
        "end": { "x": 12.0, "y": 10.0 }
      }
    ]
  },
  "dimensions": {
    "active_area_length_mm": 195.0,
    "total_routing_length_mm": 255.0,
    "board_width_mm": 20.0,
    "phases": 3,
    "magnet_array_span_mm": 120.0,
    "pole_pitch_mm": 12.0,
    "period_pitch_mm": 12.0,
    "period_count": 20,
    "phase_band_pitch_mm": 4.0,
    "phase_clearance_mm": 0.127,
    "max_phase_band_width_mm": 3.873,
    "phase_band_widths": [
      {
        "layer": 0,
        "net": "A",
        "trace_count": 5,
        "trace_width_mm": 0.127,
        "trace_spacing_mm": 0.127,
        "angle_rad": 1.030377,
        "band_width_mm": 1.333,
        "max_band_width_mm": 3.873,
        "margin_mm": 2.540
      }
    ]
  }
}
```

| Dimension field | Type | Meaning |
| --- | --- | --- |
| `active_area_length_mm` | `f64` | Active copper length from the context [mm]. |
| `total_routing_length_mm` | `f64` | Active length plus both padding ends [mm]. |
| `board_width_mm` | `f64` | Across-board width used for the braid angle [mm]. |
| `magnet_array_span_mm` | `f64?` | Full mover magnet-array span [mm], when supplied. |
| `pole_pitch_mm` | `f64?` | Centre-to-centre adjacent North/South pole pitch (`tau_p`) [mm]. |
| `period_pitch_mm` | `f64?` | Pattern repeat pitch [mm]; exact pole pitch for magnet-aware infinity braid. |
| `period_count` | `u32?` | Complete repeat periods emitted, when known. |
| `phase_band_pitch_mm` | `f64?` | Ideal phase-band pitch, `pole_pitch / phases` [mm]. |
| `phase_clearance_mm` | `f64` | `g_phase`, currently the core minimum spacing rule [mm]. |
| `max_phase_band_width_mm` | `f64?` | `pole_pitch / phases - phase_clearance` [mm]. |
| `phase_band_widths` | array | Per-active `(layer, net)` bottom-up width records. |
| `pole_regions` | array | Pattern-defined start/end boundaries for each phase and pole pitch [mm]. |

Each `phase_band_widths[]` record includes `trace_count` (`N`), `trace_width_mm`
(`w_t`), `trace_spacing_mm` (`s`), `angle_rad` (`theta`), `band_width_mm`, the
top-down maximum, and `margin_mm`.

`pole_pitch_mm` is the centre-to-centre distance between adjacent North and
South poles (`tau_p`), not the physical magnet width. `phase_band_pitch_mm` is
the ideal phase-band pitch `tau_p / phases`; it is separate from the conductor
band width in each `phase_band_widths[]` record.

`pole_regions[]` is the authoritative region interface for magnet/pole
placement. It is deliberately emitted by the pattern rather than inferred by
the host. Each record has a phase label, a zero-based `pole_index`, and
millimetre `start`/`end` points. The infinity braid emits one region per phase
per pole pitch. Its shared boundary between adjacent regions is the midpoint
between the rightmost point-3 of one diamond period and the leftmost point-1
of the next; that point is both the preceding region's end and the following
region's start. Point 0 (the top vertex) is not used as the pole boundary.
The first and last regions are centered/extrapolated from the neighboring
median spacing so they have the same width as the interior regions rather than
being clipped to the first or last generated point.

For each active `(layer, net)` group, the bottom-up width is calculated as:

```text
w_s = (N * w_t + (N - 1) * s) / sin(theta)
```

where `N` is `trace_count`, `w_t` is `trace_width_mm`, `s` is
`trace_spacing_mm`, and `theta` (`angle_rad`) is measured from the travel axis.
The top-down limit is:

```text
max(w_s) = tau_p / phases - g_phase
```

where `g_phase` is `phase_clearance_mm` (the core's minimum spacing rule). A
negative `margin_mm` is a diagnostic that the requested bundle does not fit; the
host never alters the pattern's coordinates to hide it. If a context has no
magnet pitch, the bottom-up records are still available and the top-down fields
are `null`.

The bundled infinity braid reports an exact `period_pitch_mm == pole_pitch_mm`
when magnet data is present. It reserves one uniform phase/strand interleave
step per pole-pitched period, then uses the largest number of complete periods
that fit; it never rounds the period and stretches it or leaves a wide via gap
between adjacent period grids.

### 4.2 Downstream projection: `PhaseCoil` / `CoilPathIpc`

`routing_result_to_phase_coils` groups validated elements by `(layer, net)` into
`PhaseCoil` (segments + `corner_arcs` + `center_via_positions`; each arc carries
`is_active`). The frontend DTO (`CoilPathIpc`, mirrored by
`app/src/lib/types/coils.ts`) ships the same three primitive families to the
preview:

| Wire field         | Rendered as (SVG preview)                  |
| ------------------ | ------------------------------------------ |
| `segments`         | `<line>` — thick solid for active, thin dashed for end-turns |
| `corner_arcs`      | `<path d="M s Q m e">` — dashed unless `is_active` |
| `via_positions`    | `<circle>` at via centers                  |
| `routing_dimensions` | Pole pitch, phase-band budget, and phase-band width sidecar |

The routing crate's units remain millimetres. The application IPC adapter
converts the sidecar and geometry to its existing SI/meter frontend contract;
the preview applies its own world→pixel fit including a
per-layer schematic `yOffset` (1 mm per layer) purely for readability — the
wire format itself carries no artificial offsets.

## 5. Version policy

- The current contract version is `FORMAT_VERSION = 2`
  (`crates/pcbmotorgen-routing/src/model.rs`).
- `RoutingResult.format_version` is **serde-defaulted** to the current version:
  payloads and Python runners that omit the field are interpreted as version 2
  only when they already use millimetres.
- **Additive change** (new optional field, new element type, or the host-side
  `RoutingReport` dimensions sidecar): allowed without a version bump;
  consumers must tolerate the missing field (serde `default`). The sidecar is
  not part of the strict plugin JSON.
- **Breaking change** (field removed/reinterpreted, units or coordinate
  conventions changed, validation semantics changed): MUST bump
  `format_version` and update this document. Version 2 is the millimetre
  contract; version 1 metre payloads must not be mixed into version 2.

## 6. Validation contract (summary)

All outputs — whatever their source — pass the same strict-shape
`Validator::validate(&RoutingResult, &RoutingContext)` gate before they can be
written, previewed, or simulated. Key rules:

- every coordinate finite (`NaN`/`Inf` rejected),
- coordinates inside `[0, active_area_length_mm + 2·padding_mm]` × `[0, board_width_mm]`,
  with a DFM clearance guard against board edges,
- `layer`/`from_layer`/`to_layer` inside `[0, num_layers)`,
- segments non-degenerate (nonzero length), arcs non-degenerate,
- `net` non-empty ASCII; duplicates allowed only for parallel conductors,
- continuity (end→start within tolerance) enforced when
  `expects_continuous` is set.

A non-conforming document is **rejected with a structured field-level error,
never sanitised**. Full details: `docs/adr/0009-routing-pattern-plugin-interface.md`.

## 7. Delivery mechanisms

| Source                      | Mechanism                                                       |
| --------------------------- | --------------------------------------------------------------- |
| Bundled pattern             | `RoutingPattern` trait, same process (`patterns/infinity`)      |
| Rust crate plugin           | `cdylib` C-ABI (`pcbmotorgen_routing_plugin_create`), loaded via `libloading` |
| Python runner               | subprocess: receives **flattened `RoutingContext` JSON** on stdin, emits **`RoutingResult` JSON** on stdout (nothing else), then passes the same validator |

## 8. Field references

- Rust model: `crates/pcbmotorgen-routing/src/model.rs`
- Context: `crates/pcbmotorgen-routing/src/context.rs`
- Validator: `crates/pcbmotorgen-routing/src/validator.rs`
- Adapter: `crates/pcbmotorgen-routing/src/generate/adapters.rs`
- Dimensions: `crates/pcbmotorgen-routing/src/dimensions.rs`
- Enriched report: `crates/pcbmotorgen-routing/src/report.rs`
- IPC DTO: `app/src-tauri/src/ipc/coils.rs` + `app/src/lib/types/coils.ts`
- ADR: `docs/adr/0009-routing-pattern-plugin-interface.md`
- Authoring guide: `docs/routing-pattern-authoring.md`
- README authoring: `docs/routing-readme-guide.md`
