//! IPC magnet-grade reference (`MagnetGradeIpc`) built from the core's static
//! table + the PRODUCT_GOALS thermal-suffix data.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// ===========================================================================
// Magnet grade reference (get_magnet_grades)
// ===========================================================================

/// NdFeB magnet grade specification (PRODUCT_GOALS.md §3.C).
/// `max_temp_c` maps thermal-suffix labels to °C.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MagnetGradeIpc {
    pub name: String,
    pub br_min_t: f64,
    pub br_typ_t: f64,
    pub br_max_t: f64,
    pub max_temp_c: BTreeMap<String, f64>,
}

/// Standard thermal-suffix table (PRODUCT_GOALS.md §3.C).
/// Applied to all grades except N52 (Std only).
fn standard_temp_table() -> BTreeMap<String, f64> {
    let mut m = BTreeMap::new();
    m.insert("Std".into(), 80.0);
    m.insert("H".into(), 120.0);
    m.insert("SH".into(), 150.0);
    m.insert("UH".into(), 180.0);
    m.insert("EH".into(), 200.0);
    m.insert("AH".into(), 220.0);
    m
}

/// N52 only carries the standard (no high-temp suffixes).
fn n52_temp_table() -> BTreeMap<String, f64> {
    let mut m = BTreeMap::new();
    m.insert("Std".into(), 80.0);
    m
}

/// Build the full magnet-grade list from the core's static table + the
/// PRODUCT_GOALS thermal-suffix data. This is a REAL implementation (not a
/// stub) — it reads `pcbmotorgen_simulation::magnet_grades::MAGNET_GRADES`.
pub fn magnet_grades() -> Vec<MagnetGradeIpc> {
    pcbmotorgen_simulation::magnet_grades::MAGNET_GRADES
        .iter()
        .map(|(name, br_min, br_typ, br_max)| MagnetGradeIpc {
            name: name.to_string(),
            br_min_t: *br_min,
            br_typ_t: *br_typ,
            br_max_t: *br_max,
            max_temp_c: if *name == "N52" {
                n52_temp_table()
            } else {
                standard_temp_table()
            },
        })
        .collect()
}