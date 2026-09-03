//! Client-side edge-cut bounding-box math for board diagnostics.
//!
//! The KiCad 10 IPC has no `GetBoardBounds` command, but `GetItems` with the
//! `KOT_PCB_SHAPE` type returns every board graphic (each a
//! [`BoardGraphicShape`] carrying its [`BoardLayer`] and full geometry). This
//! module filters those shapes down to the `Edge.Cuts` layer and computes the
//! axis-aligned bounding box exactly, so the diagnostics bounding box is real
//! geometry rather than a placeholder.
//!
//! Everything here is pure (no IPC) and works in **nanometres** (`f64`) — the
//! unit the IPC `Vector2` uses — so callers can union without rounding.
//! Conversion to millimetres happens once, in the caller.
//!
//! Per-shape geometry handling:
//! - **Segment** — endpoints.
//! - **Rectangle** — both corners (min/max, order-agnostic).
//! - **Circle** — centre ± radius (radius = centre → `radius_point`).
//! - **Arc** — the three points (start/mid/end), plus any of the four axis
//!   extreme points (centre ± r on each axis) the swept angle range passes
//!   through. An arc's extremes are *not* generally at its endpoints (e.g. the
//!   top of an upper semicircle), so this case needs real math.
//! - **Bezier** — endpoints plus interior extrema: the cubic's derivative is
//!   quadratic per axis, so roots are solved analytically. (The control-point
//!   convex hull would over-approximate; we compute the exact box.)
//! - **Polygon** (`PolySet`) — every node of each outline; arc nodes
//!   (`ArcStartMidEnd`) go through the same arc math. Holes are inside their
//!   outline and cannot grow the box, so they are ignored.

use crate::proto::board::types::{BoardGraphicShape, BoardLayer};
use crate::proto::common::types::{GraphicShape, Vector2};

/// Axis-aligned bounding box in nanometres (`f64` for exact arc/bezier math).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BBoxNm {
    pub x_min: f64,
    pub y_min: f64,
    pub x_max: f64,
    pub y_max: f64,
}

impl BBoxNm {
    fn from_point(x: f64, y: f64) -> Self {
        Self {
            x_min: x,
            y_min: y,
            x_max: x,
            y_max: y,
        }
    }

    fn grow_point(&mut self, x: f64, y: f64) {
        self.x_min = self.x_min.min(x);
        self.y_min = self.y_min.min(y);
        self.x_max = self.x_max.max(x);
        self.y_max = self.y_max.max(y);
    }

    fn grow(&mut self, other: &BBoxNm) {
        self.grow_point(other.x_min, other.y_min);
        self.grow_point(other.x_max, other.y_max);
    }
}

/// `BoardLayer::BL_Edge_Cuts` as the wire enum value.
const EDGE_CUTS_LAYER: i32 = BoardLayer::BlEdgeCuts as i32;

/// Bounding box of all edge-cut graphics in `shapes`, or `None` when the list
/// contains no usable edge-cut geometry (no edge cuts, or geometry the API
/// did not populate).
pub(crate) fn edge_cut_bbox_nm(shapes: &[BoardGraphicShape]) -> Option<BBoxNm> {
    let mut acc: Option<BBoxNm> = None;
    for shape in shapes {
        if shape.layer != EDGE_CUTS_LAYER {
            continue;
        }
        let Some(graphic) = shape.shape.as_ref() else {
            continue;
        };
        if let Some(bbox) = graphic_shape_bbox(graphic) {
            match acc.as_mut() {
                Some(acc) => acc.grow(&bbox),
                None => acc = Some(bbox),
            }
        }
    }
    acc
}

/// Bounding box of one board graphic shape, or `None` if its geometry is
/// absent / unhandled.
fn graphic_shape_bbox(shape: &GraphicShape) -> Option<BBoxNm> {
    match shape.geometry.as_ref()? {
        crate::proto::common::types::graphic_shape::Geometry::Segment(seg) => {
            let (start, end) = (seg.start.as_ref()?, seg.end.as_ref()?);
            let mut b = BBoxNm::from_point(start.x_nm as f64, start.y_nm as f64);
            b.grow_point(end.x_nm as f64, end.y_nm as f64);
            Some(b)
        }
        crate::proto::common::types::graphic_shape::Geometry::Rectangle(rect) => {
            let (tl, br) = (rect.top_left.as_ref()?, rect.bottom_right.as_ref()?);
            let mut b = BBoxNm::from_point(tl.x_nm as f64, tl.y_nm as f64);
            b.grow_point(br.x_nm as f64, br.y_nm as f64);
            Some(b)
        }
        crate::proto::common::types::graphic_shape::Geometry::Arc(arc) => {
            let (start, mid, end) = (arc.start.as_ref()?, arc.mid.as_ref()?, arc.end.as_ref()?);
            Some(arc_bbox(start, mid, end))
        }
        crate::proto::common::types::graphic_shape::Geometry::Circle(circle) => {
            let (center, radius_point) = (circle.center.as_ref()?, circle.radius_point.as_ref()?);
            let (cx, cy) = (center.x_nm as f64, center.y_nm as f64);
            let r = ((radius_point.x_nm - center.x_nm) as f64).hypot((radius_point.y_nm - center.y_nm) as f64);
            Some(BBoxNm {
                x_min: cx - r,
                y_min: cy - r,
                x_max: cx + r,
                y_max: cy + r,
            })
        }
        crate::proto::common::types::graphic_shape::Geometry::Polygon(polyset) => {
            let mut acc: Option<BBoxNm> = None;
            for poly in &polyset.polygons {
                // Holes sit inside their outline and cannot grow the box;
                // only outlines contribute.
                let Some(outline) = poly.outline.as_ref() else {
                    continue;
                };
                for node in &outline.nodes {
                    let bbox = match node.geometry.as_ref()? {
                        crate::proto::common::types::poly_line_node::Geometry::Point(p) => {
                            BBoxNm::from_point(p.x_nm as f64, p.y_nm as f64)
                        }
                        crate::proto::common::types::poly_line_node::Geometry::Arc(arc) => {
                            arc_bbox(arc.start.as_ref()?, arc.mid.as_ref()?, arc.end.as_ref()?)
                        }
                    };
                    match acc.as_mut() {
                        Some(acc) => acc.grow(&bbox),
                        None => acc = Some(bbox),
                    }
                }
            }
            acc
        }
        crate::proto::common::types::graphic_shape::Geometry::Bezier(bez) => {
            let (p0, p1, p2, p3) = (
                bez.start.as_ref()?,
                bez.control1.as_ref()?,
                bez.control2.as_ref()?,
                bez.end.as_ref()?,
            );
            Some(bezier_bbox(p0, p1, p2, p3))
        }
    }
}

/// Bounding box of the circular arc through `start` → `mid` → `end`.
///
/// Computes the circumcircle of the three points, decides the sweep direction
/// from where `mid` lies, and unions in any axis extreme points (centre ± r)
/// the sweep crosses. Degenerate input (collinear or coincident points) falls
/// back to the bounding box of the three points themselves.
fn arc_bbox(start: &Vector2, mid: &Vector2, end: &Vector2) -> BBoxNm {
    let (ax, ay) = (start.x_nm as f64, start.y_nm as f64);
    let (bx, by) = (mid.x_nm as f64, mid.y_nm as f64);
    let (cx, cy) = (end.x_nm as f64, end.y_nm as f64);

    let mut bbox = BBoxNm::from_point(ax, ay);
    bbox.grow_point(bx, by);
    bbox.grow_point(cx, cy);

    // Circumcentre via the perpendicular-bisector formula, with i128
    // intermediates so the integer products stay exact before the final
    // division (nm coordinates squared reach ~1e16, times coordinates ~1e24).
    let d_x2 = 2 * (ax as i128 * (by as i128 - cy as i128)
        + bx as i128 * (cy as i128 - ay as i128)
        + cx as i128 * (ay as i128 - by as i128));
    if d_x2 == 0 {
        // Collinear (or coincident) points — no circle exists; the three
        // points themselves bound the degenerate "arc".
        return bbox;
    }
    let (a2, b2, c2) = (
        ax as i128 * ax as i128 + ay as i128 * ay as i128,
        bx as i128 * bx as i128 + by as i128 * by as i128,
        cx as i128 * cx as i128 + cy as i128 * cy as i128,
    );
    let ux = (a2 * (by as i128 - cy as i128)
        + b2 * (cy as i128 - ay as i128)
        + c2 * (ay as i128 - by as i128)) as f64
        / d_x2 as f64;
    let uy = (a2 * (cx as i128 - bx as i128)
        + b2 * (ax as i128 - cx as i128)
        + c2 * (bx as i128 - ax as i128)) as f64
        / d_x2 as f64;

    let r = (ax - ux).hypot(ay - uy);
    if r <= 0.0 {
        return bbox;
    }

    let tau = std::f64::consts::TAU;
    let a0 = (ay - uy).atan2(ax - ux); // start angle
    let a1 = (by - uy).atan2(bx - ux); // mid angle
    let a2_ang = (cy - uy).atan2(cx - ux); // end angle

    // `norm` maps an angle difference into [0, tau).
    let norm = |mut d: f64| {
        d %= tau;
        if d < 0.0 {
            d += tau;
        }
        d
    };

    // CCW sweep from start to end; `mid` decides the true direction.
    let sweep_ccw = norm(a2_ang - a0);
    let mid_ccw = norm(a1 - a0);
    let ccw = mid_ccw <= sweep_ccw;
    let sweep = if ccw { sweep_ccw } else { tau - sweep_ccw };

    // Axis extreme points the sweep passes through.
    for (k, ex, ey) in [
        (0.0, ux + r, uy),
        (std::f64::consts::FRAC_PI_2, ux, uy + r),
        (std::f64::consts::PI, ux - r, uy),
        (3.0 * std::f64::consts::FRAC_PI_2, ux, uy - r),
    ] {
        let reached = if ccw {
            norm(k - a0) <= sweep
        } else {
            norm(a0 - k) <= sweep
        };
        if reached {
            bbox.grow_point(ex, ey);
        }
    }

    bbox
}

/// Exact bounding box of the cubic bezier through `p0..p3`.
///
/// Per axis, `B'(t)/3 = a·t² + b·t + c` with `a = p3 − 3p2 + 3p1 − p0`,
/// `b = 2(p0 − 2p1 + p2)`, `c = p1 − p0`; interior extrema are the roots in
/// (0, 1), solved analytically.
fn bezier_bbox(p0: &Vector2, p1: &Vector2, p2: &Vector2, p3: &Vector2) -> BBoxNm {
    let px = [
        p0.x_nm as f64,
        p1.x_nm as f64,
        p2.x_nm as f64,
        p3.x_nm as f64,
    ];
    let py = [
        p0.y_nm as f64,
        p1.y_nm as f64,
        p2.y_nm as f64,
        p3.y_nm as f64,
    ];

    let mut bbox = BBoxNm::from_point(px[0], py[0]);
    bbox.grow_point(px[3], py[3]);

    // Candidate parameters: endpoints plus interior derivative roots.
    let mut ts = vec![0.0f64, 1.0f64];
    for (a, b, c) in [
        (
            px[3] - 3.0 * px[2] + 3.0 * px[1] - px[0],
            2.0 * (px[0] - 2.0 * px[1] + px[2]),
            px[1] - px[0],
        ),
        (
            py[3] - 3.0 * py[2] + 3.0 * py[1] - py[0],
            2.0 * (py[0] - 2.0 * py[1] + py[2]),
            py[1] - py[0],
        ),
    ] {
        let roots = if a.abs() < 1e-9 {
            // Degenerate to linear (or constant) derivative.
            if b.abs() < 1e-9 {
                Vec::new()
            } else {
                vec![-c / b]
            }
        } else {
            let disc = b * b - 4.0 * a * c;
            if disc < 0.0 {
                Vec::new()
            } else {
                let sq = disc.sqrt();
                vec![(-b + sq) / (2.0 * a), (-b - sq) / (2.0 * a)]
            }
        };
        for t in roots {
            if (0.0..=1.0).contains(&t) {
                ts.push(t);
            }
        }
    }

    for t in ts {
        let u = 1.0 - t;
        // B(t) = u³·p0 + 3u²t·p1 + 3ut²·p2 + t³·p3
        let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
        let x = w[0] * px[0] + w[1] * px[1] + w[2] * px[2] + w[3] * px[3];
        let y = w[0] * py[0] + w[1] * py[1] + w[2] * py[2] + w[3] * py[3];
        bbox.grow_point(x, y);
    }

    bbox
}

// ---------------------------------------------------------------------------
// Tests (pure geometry — hand-computed expectations, no IPC)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::board::types::Net;
    use crate::proto::common::types::{
        GraphicArcAttributes, GraphicBezierAttributes, GraphicCircleAttributes,
        GraphicRectangleAttributes, GraphicSegmentAttributes, PolyLine, PolyLineNode, PolySet,
        PolygonWithHoles, graphic_shape::Geometry as Geom,
    };

    const MM: f64 = 1e6; // nm per mm

    fn vec2(x_mm: f64, y_mm: f64) -> Option<Vector2> {
        Some(Vector2 {
            x_nm: (x_mm * MM).round() as i64,
            y_nm: (y_mm * MM).round() as i64,
        })
    }

    /// Polyline point nodes hold a bare `Vector2` (not `Option`).
    fn node2(x_mm: f64, y_mm: f64) -> Vector2 {
        Vector2 {
            x_nm: (x_mm * MM).round() as i64,
            y_nm: (y_mm * MM).round() as i64,
        }
    }

    /// Wraps a oneof geometry variant into an edge-cut board shape.
    fn edge_shape(geometry: Geom) -> BoardGraphicShape {
        BoardGraphicShape {
            shape: Some(GraphicShape {
                attributes: None,
                geometry: Some(geometry),
            }),
            layer: BoardLayer::BlEdgeCuts as i32,
            net: Some(Net { code: None, name: String::new() }),
            id: None,
            locked: 0,
        }
    }

    fn shape_on(layer: BoardLayer, geometry: Geom) -> BoardGraphicShape {
        let mut s = edge_shape(geometry);
        s.layer = layer as i32;
        s
    }

    fn segment(x0: f64, y0: f64, x1: f64, y1: f64) -> Geom {
        Geom::Segment(GraphicSegmentAttributes {
            start: vec2(x0, y0),
            end: vec2(x1, y1),
        })
    }

    fn assert_close(bbox: BBoxNm, x_min: f64, y_min: f64, x_max: f64, y_max: f64) {
        for (got, want, label) in [
            (bbox.x_min, x_min, "x_min"),
            (bbox.y_min, y_min, "y_min"),
            (bbox.x_max, x_max, "x_max"),
            (bbox.y_max, y_max, "y_max"),
        ] {
            assert!(
                (got - want).abs() < 1.0, // 1 nm tolerance
                "{label}: got {got} nm, want {want} nm"
            );
        }
    }

    #[test]
    fn segment_bbox_is_endpoints() {
        let bbox = edge_cut_bbox_nm(&[edge_shape(segment(0.0, 2.0, 4.0, -6.0))]).unwrap();
        assert_close(bbox, 0.0, -6.0 * MM, 4.0 * MM, 2.0 * MM);
    }

    #[test]
    fn rectangle_bbox_is_corners() {
        let rect = Geom::Rectangle(GraphicRectangleAttributes {
            top_left: vec2(-2.0, 5.0),
            bottom_right: vec2(3.0, -1.0),
            corner_radius: Some(crate::proto::common::types::Distance { value_nm: 0 }),
        });
        let bbox = edge_cut_bbox_nm(&[edge_shape(rect)]).unwrap();
        assert_close(bbox, -2.0 * MM, -1.0 * MM, 3.0 * MM, 5.0 * MM);
    }

    #[test]
    fn circle_bbox_is_center_plus_radius() {
        let circle = Geom::Circle(GraphicCircleAttributes {
            center: vec2(2.0, 3.0),
            radius_point: vec2(3.0, 3.0),
        });
        let bbox = edge_cut_bbox_nm(&[edge_shape(circle)]).unwrap();
        assert_close(bbox, 1.0 * MM, 2.0 * MM, 3.0 * MM, 4.0 * MM);
    }

    #[test]
    fn upper_semicircle_arc_includes_top_extreme() {
        // Arc from (0,0) over the top (5,5) to (10,0): centre (5,0), r 5.
        // The topmost point (5,5) is an axis extreme, not an endpoint.
        let arc = Geom::Arc(GraphicArcAttributes {
            start: vec2(0.0, 0.0),
            mid: vec2(5.0, 5.0),
            end: vec2(10.0, 0.0),
        });
        let bbox = edge_cut_bbox_nm(&[edge_shape(arc)]).unwrap();
        assert_close(bbox, 0.0, 0.0, 10.0 * MM, 5.0 * MM);
    }

    #[test]
    fn lower_semicircle_arc_includes_bottom_extreme() {
        // Arc from (0,0) under the bottom (5,-5) to (10,0).
        let arc = Geom::Arc(GraphicArcAttributes {
            start: vec2(0.0, 0.0),
            mid: vec2(5.0, -5.0),
            end: vec2(10.0, 0.0),
        });
        let bbox = edge_cut_bbox_nm(&[edge_shape(arc)]).unwrap();
        assert_close(bbox, 0.0, -5.0 * MM, 10.0 * MM, 0.0);
    }

    #[test]
    fn minor_arc_without_axis_crossing_bounded_by_points() {
        // Centre (0,0), r 5 mm: start (3,4) [53.13°], mid (4,3) [36.87°],
        // end (5,0) [0°] — the sweep touches no axis extreme beyond the
        // endpoint at 0°.
        let arc = Geom::Arc(GraphicArcAttributes {
            start: vec2(3.0, 4.0),
            mid: vec2(4.0, 3.0),
            end: vec2(5.0, 0.0),
        });
        let bbox = edge_cut_bbox_nm(&[edge_shape(arc)]).unwrap();
        assert_close(bbox, 3.0 * MM, 0.0, 5.0 * MM, 4.0 * MM);
    }

    #[test]
    fn collinear_arc_falls_back_to_points() {
        let arc = Geom::Arc(GraphicArcAttributes {
            start: vec2(0.0, 0.0),
            mid: vec2(5.0, 0.0),
            end: vec2(10.0, 0.0),
        });
        let bbox = edge_cut_bbox_nm(&[edge_shape(arc)]).unwrap();
        assert_close(bbox, 0.0, 0.0, 10.0 * MM, 0.0);
    }

    #[test]
    fn bezier_interior_extremum_beats_control_hull() {
        // x(t) = 30·t(1−t) mm peaks at t=0.5 → 7.5 mm, while the control
        // points span [0, 10]; the exact box must be [0, 7.5] × [0, 10].
        let bez = Geom::Bezier(GraphicBezierAttributes {
            start: vec2(0.0, 0.0),
            control1: vec2(10.0, 0.0),
            control2: vec2(10.0, 10.0),
            end: vec2(0.0, 10.0),
        });
        let bbox = edge_cut_bbox_nm(&[edge_shape(bez)]).unwrap();
        assert_close(bbox, 0.0, 0.0, 7.5 * MM, 10.0 * MM);
    }

    #[test]
    fn polygon_outline_nodes_and_arc_nodes_are_unioned() {
        // Outline: (0,0) → (2,0) → arc up through (3,1) to (2,2) → closed.
        // The arc has centre (2,1), r 1; it reaches x = 3 at its mid point.
        let polyset = PolySet {
            polygons: vec![PolygonWithHoles {
                outline: Some(PolyLine {
                    nodes: vec![
                        PolyLineNode {
                            geometry: Some(crate::proto::common::types::poly_line_node::Geometry::Point(
                                node2(0.0, 0.0),
                            )),
                        },
                        PolyLineNode {
                            geometry: Some(crate::proto::common::types::poly_line_node::Geometry::Point(
                                node2(2.0, 0.0),
                            )),
                        },
                        PolyLineNode {
                            geometry: Some(crate::proto::common::types::poly_line_node::Geometry::Arc(
                                crate::proto::common::types::ArcStartMidEnd {
                                    start: vec2(2.0, 0.0),
                                    mid: vec2(3.0, 1.0),
                                    end: vec2(2.0, 2.0),
                                },
                            )),
                        },
                        PolyLineNode {
                            geometry: Some(crate::proto::common::types::poly_line_node::Geometry::Point(
                                node2(0.0, 2.0),
                            )),
                        },
                    ],
                    closed: true,
                }),
                holes: Vec::new(),
            }],
        };
        let bbox = edge_cut_bbox_nm(&[edge_shape(Geom::Polygon(polyset))]).unwrap();
        assert_close(bbox, 0.0, 0.0, 3.0 * MM, 2.0 * MM);
    }

    #[test]
    fn non_edge_cut_layers_are_ignored() {
        let shapes = vec![
            shape_on(BoardLayer::BlFSilkS, segment(0.0, 0.0, 1.0, 1.0)),
            shape_on(BoardLayer::BlDwgsUser, segment(2.0, 0.0, 3.0, 0.0)),
        ];
        assert_eq!(edge_cut_bbox_nm(&shapes), None);
    }

    #[test]
    fn empty_and_geometryless_shapes_yield_none() {
        assert_eq!(edge_cut_bbox_nm(&[]), None);
        // Edge-cut shape with no geometry payload must be skipped, not panic.
        assert_eq!(
            edge_cut_bbox_nm(&[BoardGraphicShape {
                shape: None,
                layer: EDGE_CUTS_LAYER,
                net: None,
                id: None,
                locked: 0,
            }]),
            None
        );
    }

    #[test]
    fn multiple_shapes_are_unioned() {
        let shapes = vec![
            edge_shape(segment(0.0, 0.0, 4.0, 1.0)),
            edge_shape(segment(-2.0, -3.0, 1.0, 8.0)),
        ];
        let bbox = edge_cut_bbox_nm(&shapes).unwrap();
        assert_close(bbox, -2.0 * MM, -3.0 * MM, 4.0 * MM, 8.0 * MM);
    }
}
