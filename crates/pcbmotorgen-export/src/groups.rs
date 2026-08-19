//! DXF group-code / value pair codec.
//!
//! Every DXF record is a group `code` line followed by a `value` line.
//! These helpers append `"code\nvalue"` pairs to the shared fragment
//! buffer; the entry points join the fragments with newlines to produce
//! the final ASCII DXF file.

/// Append a single DXF group-code / value pair to the fragment buffer.
pub(crate) fn dxf_group(out: &mut Vec<String>, code: i32, value: &str) {
    out.push(format!("{code}\n{value}"));
}

/// Append a single DXF group-code / value pair, formatting a float value.
///
/// - Clamps tiny values to exact zero to prevent `-0.000000` in output.
/// - DXF R12 uses up to 16 decimal digits; 6 is enough for 0.001 mm
///   precision.
pub(crate) fn dxf_group_f64(out: &mut Vec<String>, code: i32, value: f64) {
    let v = if value.abs() < 1e-12 { 0.0 } else { value };
    dxf_group(out, code, &format!("{v:.6}"));
}
