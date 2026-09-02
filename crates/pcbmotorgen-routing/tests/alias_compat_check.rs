//! Compat: pre-rename slot-width payload keys still deserialize via the serde
//! aliases documented in docs/API.md §15 (version policy).
use pcbmotorgen_routing::RoutingDimensions;

#[test]
fn old_slot_width_payload_keys_deserialize() {
    let legacy = r#"{
        "active_area_length_mm": 195.0,
        "total_routing_length_mm": 255.0,
        "board_width_mm": 20.0,
        "phases": 3,
        "pole_pitch_mm": 12.0,
        "slot_pitch_mm": 4.0,
        "phase_clearance_mm": 0.127,
        "max_slot_width_mm": 3.873,
        "slot_widths": [
            {
                "layer": 0,
                "net": "A",
                "trace_count": 5,
                "trace_width_mm": 0.127,
                "trace_spacing_mm": 0.127,
                "angle_rad": 1.030377,
                "slot_width_mm": 1.333,
                "max_slot_width_mm": 3.873,
                "margin_mm": 2.540
            }
        ]
    }"#;
    let dims: RoutingDimensions = serde_json::from_str(legacy).expect("legacy payload deserializes");
    assert_eq!(dims.phase_band_pitch_mm, Some(4.0));
    assert_eq!(dims.max_phase_band_width_mm, Some(3.873));
    assert_eq!(dims.phase_band_widths.len(), 1);
    let band = &dims.phase_band_widths[0];
    assert_eq!(band.band_width_mm, 1.333);
    assert_eq!(band.max_band_width_mm, Some(3.873));
    assert!(dims.all_phase_bands_fit());
}
