# pcbmotorgen-export

A library that exports PCB designs to various formats.

## Building / testing

```bash
cargo build -p pcbmotorgen-export
cargo test  -p pcbmotorgen-export     # 100+ unit + integration tests (offline, no KiCad needed)
cargo doc   -p pcbmotorgen-export     # rustdoc for the whole public API
```

The KiCad `.proto` schema is re-synced with `scripts/sync_protos.sh`
(regenerates the bindings at build time via `protox` + `prost-build`).

[`pcbmotorgen-routing`]: ../pcbmotorgen-routing/
[`pcbmotorgen-dfm`]: ../pcbmotorgen-dfm/
[`PhaseCoil`]: ../pcbmotorgen-routing/src/coil.rs
[`RoutingResult`]: ../pcbmotorgen-routing/src/coil.rs
[`DesignRules`]: ../pcbmotorgen-dfm/src/rules.rs
