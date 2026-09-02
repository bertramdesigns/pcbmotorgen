//! Compat: the pre-rename slot-width payload keys now bind to the TRUE
//! per-slot fields (kata mqw4).
//!
//! History: the v-next terminology alignment added serde aliases so payloads
//! using the old misnomer keys (`slot_pitch_mm` for the phase-band pitch,
//! per-band `slot_width_mm` for the band width) kept deserializing. With the
//! glossary-exact per-slot metrics those two key names became real fields with
//! different meanings, so the aliases were retired:
//!
//! - `RoutingDimensions.slot_pitch_mm` is now the true slot pitch
//!   `tau_s = L_stator / N_slots` (not the phase-band pitch
//!   `phase_band_pitch_mm`);
//! - `PhaseBandWidth.slot_width_mm` is now the single-leg slot width (not the
//!   band bundle width `band_width_mm`).
//!
//! Payloads that still use the old keys therefore populate the new slot
//! fields. The band record requires `band_width_mm`; a payload that only
//! carries the old per-band `slot_width_mm` key is rejected instead of being
//! silently reinterpreted.
use pcbmotorgen_routing::RoutingDimensions;

#[test]
fn slot_key_names_now_bind_to_the_true_per_slot_fields() {
    let payload = r#"{
        "active_area_length_mm": 195.0,
        "total_routing_length_mm": 255.0,
        "board_width_mm": 20.0,
        "phases": 3,
        "pole_pitch_mm": 12.0,
        "phase_band_pitch_mm": 4.0,
        "phase_clearance_mm": 0.127,
        "max_slot_width_mm": 3.873,
        "slot_pitch_mm": 0.8,
        "slot_count": 975,
        "interleave_step_mm": 0.8,
        "phase_band_widths": [
            {
                "layer": 0,
                "net": "A",
                "trace_count": 5,
                "trace_width_mm": 0.127,
                "trace_spacing_mm": 0.127,
                "angle_rad": 1.030377,
                "band_width_mm": 1.333,
                "slot_width_mm": 0.148,
                "max_slot_width_mm": 3.873,
                "margin_mm": 2.540
            }
        ]
    }"#;
    let dims: RoutingDimensions = serde_json::from_str(payload).expect("payload deserializes");
    // The true per-slot fields bind to their glossary-exact names ...
    assert_eq!(dims.slot_pitch_mm, Some(0.8));
    assert_eq!(dims.slot_count, Some(975));
    assert_eq!(dims.interleave_step_mm, Some(0.8));
    assert_eq!(dims.phase_band_widths[0].slot_width_mm, Some(0.148));
    // ... and stay distinct from the phase-band quantities.
    assert_eq!(dims.phase_band_pitch_mm, Some(4.0));
    assert_eq!(dims.max_phase_band_width_mm, Some(3.873));
    assert_eq!(dims.phase_band_widths[0].band_width_mm, 1.333);
    assert!(dims.all_phase_bands_fit());
}

#[test]
fn band_record_requires_band_width_mm_instead_of_silently_reinterpreting() {
    // A pre-rename payload that only carries the per-band `slot_width_mm`
    // alias no longer loads: `band_width_mm` is required, and silently
    // mapping the old key onto the new single-leg field would misreport the
    // band by a factor of `trace_count`.
    let legacy = r#"{
        "active_area_length_mm": 195.0,
        "total_routing_length_mm": 255.0,
        "board_width_mm": 20.0,
        "phases": 3,
        "phase_clearance_mm": 0.127,
        "phase_band_widths": [
            {
                "layer": 0,
                "net": "A",
                "trace_count": 5,
                "trace_width_mm": 0.127,
                "trace_spacing_mm": 0.127,
                "angle_rad": 1.030377,
                "slot_width_mm": 1.333
            }
        ]
    }"#;
    let result: Result<RoutingDimensions, _> = serde_json::from_str(legacy);
    assert!(result.is_err(), "missing band_width_mm must be rejected");
}
