# KiCad communication

This document outlines the communication between the `pcbmotorgen` Rust crate and a running instance of KiCad with the IPC protocol.

## 1. KiCad 10 IPC Protocol — Native Rust (in `pcbmotorgen-export`)

### 1.1 Transport Layer

KiCad 10 exposes its scripting/board API via the **NNG (nanomsg next
generation)** `req0` protocol over an IPC socket. The Python reference
implementation (`kipy/client.py`) uses `pynng.Req0`.

**Rust equivalent:** the `nng` crate provides `Req0` with `dial()`, `send()`,
and `recv()` semantics matching `pynng`.

| Parameter   | Default                        | Overridable by             |
| ----------- | ------------------------------ | -------------------------- |
| Socket path | `ipc:///tmp/kicad/api.sock`    | `KICAD_API_SOCKET` env var |
| Client name | `pcbmotorgen-<random-8-char>`  | Constructor argument       |
| KiCad token | `""` (empty — auto-negotiated) | `KICAD_API_TOKEN` env var  |
| Timeout     | 2000 ms                        | Constructor argument       |

### 1.2 Wire Protocol

Every request/response is a length-prefixed protobuf envelope:

```
┌──────────────────────────────────────────────────┐
│  ApiRequest                                       │
│  ├── header { kicad_token, client_name }          │
│  └── message: google.protobuf.Any (packed cmd)   │
└──────────────────────────────────────────────────┘
                        │ NNG req0.send()
                        ▼
┌──────────────────────────────────────────────────┐
│  ApiResponse                                      │
│  ├── header { kicad_token }                       │
│  ├── status { status: ApiStatusCode, error_msg }  │
│  └── message: google.protobuf.Any (packed reply)  │
└──────────────────────────────────────────────────┘
```

**Status handling:** if `status != AS_OK`, raise `KiCadError` with the
`error_message` and `status_code`. On first successful response, cache the
returned `kicad_token` for subsequent requests.

### 1.3 Protobuf Code Generation

The `.proto` schema files are vendored under `crates/pcbmotorgen-export/proto/`
and compiled at build time by `build.rs` via `protox` + `prost-build` into a
single `kiapi.rs` umbrella (`pcbmotorgen_export::proto`). `tonic` is **not**
used. `scripts/sync_protos.sh` re-copies the schema from the `kicad-python`
reference repo.

---

## 2. KiCad Writer (`pcbmotorgen_export::writer`)

The writer is **decoupled from config**: `coils_to_board_items` takes the
generic `PhaseCoil` set plus `num_layers`, a `DesignRules`, and the active-area
length.

```
PhaseCoil (from pcbmotorgen_routing::coil)
  ├── segments: Vec<CoilSegment { start, end, is_active }>
  ├── layer_idx: u32
  └── center_via_positions: Vec<(f64, f64)>
        │
        ▼
writer::coils_to_board_items(coils, num_layers, rules, active_area_length_m)
        │
        ├── straight segment → Track {
        │       width:  rules.min_trace_m → nm,
        │       layer:  layer_map(idx, num_layers),
        │       net:    /<phase>
        │   }
        └── via position → Via {
                drill:      rules.min_via_drill_m → nm,
                pad:        rules.via_pad_diameter_m() → nm,
                layer_set:  num_layers copper layers,
                net:        /<phase>
            }
        │
        ▼
BoardHandle::write_coils(...) → Commit::create_items → Commit::end (atomic)
```

Unit conversions: all geometry is in **millimetres**; the writer converts to **nm**
via `layer_map::mm_to_nm`. `layer_map::layer_idx_to_board_layer(idx, num_layers)`
maps `0 → B_Cu`, `num_layers-1 → F_Cu`, and inner indices to `In{n}_Cu`.

**Offline testability:** `coils_to_board_items` is a pure function; `client` is
behind a `KicadTransport` trait with `MockTransport`; `#[cfg(feature =
"kicad-live")]` gated integration tests require a live socket and are skipped in
CI.
