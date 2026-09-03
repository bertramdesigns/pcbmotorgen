//! Integration tests for `get_board_diagnostics` using a type-dispatching
//! mock transport (kata ze9f).
//!
//! Unlike the single-command [`MockTransport`] (which returns the same bytes
//! for every request), the diagnostics snapshot issues *several* IPC commands
//! in sequence (`GetBoardEnabledLayers`, `GetItems`, `GetNets`,
//! `GetNetClassForNets`). [`DispatchTransport`] decodes each request envelope,
//! records the command type URL, and returns the canned response registered
//! for that command — so multi-command flows can be pinned offline, including
//! which commands are (and are not) sent.
//!
//! What these tests pin:
//! - net-class population from `GetNetClassForNets` responses (multi-class,
//!   composite-class, and empty-board edges);
//! - edge-cut bounding box derivation from canned `GetItems` shape lists;
//! - the documented degradation path (failing/absent queries fall back to
//!   `0.0` / `[]` without failing the snapshot);
//! - `BoardHandle::get_board_origin` (an IPC query that exists but has no
//!   diagnostics field — pinned here so it stays real and tested).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use prost::Message;
use prost_types::Any;

use pcbmotorgen_export::proto::board::commands::{
    BoardEnabledLayersResponse, BoardOriginType, NetClassForNetsResponse, NetsResponse,
};
use pcbmotorgen_export::proto::board::types::{BoardGraphicShape, BoardLayer, Net};
use pcbmotorgen_export::proto::common::commands::GetItemsResponse;
use pcbmotorgen_export::proto::common::project::NetClass;
use pcbmotorgen_export::proto::common::types::{
    GraphicSegmentAttributes, GraphicShape, ItemRequestStatus, Vector2,
};
use pcbmotorgen_export::{
    BoardDiagnostics, BoardHandle, DocumentSpecifier, DocumentType, KiCadClient, KiCadError,
    KicadTransport, get_board_diagnostics,
};

// ---------------------------------------------------------------------------
// Command / payload type URLs
// ---------------------------------------------------------------------------

const GET_BOARD_ENABLED_LAYERS_URL: &str =
    "type.googleapis.com/kiapi.board.commands.GetBoardEnabledLayers";
const BOARD_ENABLED_LAYERS_RESPONSE_URL: &str =
    "type.googleapis.com/kiapi.board.commands.BoardEnabledLayersResponse";
const GET_ITEMS_URL: &str = "type.googleapis.com/kiapi.common.commands.GetItems";
const GET_ITEMS_RESPONSE_URL: &str =
    "type.googleapis.com/kiapi.common.commands.GetItemsResponse";
const GET_NETS_URL: &str = "type.googleapis.com/kiapi.board.commands.GetNets";
const NETS_RESPONSE_URL: &str = "type.googleapis.com/kiapi.board.commands.NetsResponse";
const GET_NET_CLASS_FOR_NETS_URL: &str =
    "type.googleapis.com/kiapi.board.commands.GetNetClassForNets";
const NETCLASS_FOR_NETS_RESPONSE_URL: &str =
    "type.googleapis.com/kiapi.board.commands.NetClassForNetsResponse";
const GET_BOARD_ORIGIN_URL: &str =
    "type.googleapis.com/kiapi.board.commands.GetBoardOrigin";
const SHAPE_URL: &str = "type.googleapis.com/kiapi.board.types.BoardGraphicShape";
const VECTOR2_URL: &str = "type.googleapis.com/kiapi.common.types.Vector2";

// ---------------------------------------------------------------------------
// Dispatching mock transport
// ---------------------------------------------------------------------------

/// Mock transport that routes each request to the canned response registered
/// for its command type URL. Requests are recorded in order.
struct DispatchTransport {
    responses: BTreeMap<String, Vec<u8>>,
    sent_type_urls: Vec<String>,
}

impl DispatchTransport {
    fn new() -> Self {
        Self {
            responses: BTreeMap::new(),
            sent_type_urls: Vec::new(),
        }
    }

    /// Registers an AS_OK response with the given payload for a command.
    fn on(mut self, type_url: &str, payload: Any) -> Self {
        let mut buf = Vec::new();
        pcbmotorgen_export::ApiResponse {
            header: None,
            status: Some(pcbmotorgen_export::ApiResponseStatus {
                status: pcbmotorgen_export::ApiStatusCode::AsOk as i32,
                error_message: String::new(),
            }),
            message: Some(payload),
        }
        .encode(&mut buf)
        .expect("encode envelope");
        self.responses.insert(type_url.to_string(), buf);
        self
    }

    /// Registers a non-OK (AS_UNHANDLED) response — the canned equivalent of
    /// a KiCad build without that command handler.
    fn unhandled(mut self, type_url: &str) -> Self {
        let mut buf = Vec::new();
        pcbmotorgen_export::ApiResponse {
            header: None,
            status: Some(pcbmotorgen_export::ApiResponseStatus {
                status: pcbmotorgen_export::ApiStatusCode::AsUnhandled as i32,
                error_message: "unhandled command".to_string(),
            }),
            message: None,
        }
        .encode(&mut buf)
        .expect("encode envelope");
        self.responses.insert(type_url.to_string(), buf);
        self
    }

    fn pack<T: Message>(type_url: &str, msg: &T) -> Any {
        let mut buf = Vec::new();
        msg.encode(&mut buf).expect("encode message");
        Any {
            type_url: type_url.to_string(),
            value: buf,
        }
    }
}

impl KicadTransport for DispatchTransport {
    fn send_and_recv(&mut self, request_bytes: &[u8]) -> Result<Vec<u8>, KiCadError> {
        let request = pcbmotorgen_export::ApiRequest::decode(request_bytes)
            .expect("mock must receive a decodable ApiRequest");
        let type_url = request.message.map(|m| m.type_url).unwrap_or_default();
        self.sent_type_urls.push(type_url.clone());
        self.responses
            .get(&type_url)
            .cloned()
            .ok_or_else(|| KiCadError::Protocol(format!("no canned response for {type_url}")))
    }
}

/// `Rc<RefCell<…>>` wrapper so the test can inspect the mock's recorded
/// requests after the client (which owns the boxed transport) has run a flow.
#[derive(Clone)]
struct SharedDispatch(Rc<RefCell<DispatchTransport>>);

impl KicadTransport for SharedDispatch {
    fn send_and_recv(&mut self, request_bytes: &[u8]) -> Result<Vec<u8>, KiCadError> {
        self.0.borrow_mut().send_and_recv(request_bytes)
    }
}

// ---------------------------------------------------------------------------
// Canned payload builders + snapshot harness
// ---------------------------------------------------------------------------

fn pcb_document() -> DocumentSpecifier {
    DocumentSpecifier {
        r#type: DocumentType::DoctypePcb as i32,
        identifier: Some(
            pcbmotorgen_export::proto::common::types::document_specifier::Identifier::BoardFilename(
                "motor.kicad_pcb".to_string(),
            ),
        ),
        project: None,
    }
}

fn edge_cut_segment(x0: f64, y0: f64, x1: f64, y1: f64) -> BoardGraphicShape {
    let nm = |mm: f64| (mm * 1e6).round() as i64;
    BoardGraphicShape {
        shape: Some(GraphicShape {
            attributes: None,
            geometry: Some(
                pcbmotorgen_export::proto::common::types::graphic_shape::Geometry::Segment(
                    GraphicSegmentAttributes {
                        start: Some(Vector2 { x_nm: nm(x0), y_nm: nm(y0) }),
                        end: Some(Vector2 { x_nm: nm(x1), y_nm: nm(y1) }),
                    },
                ),
            ),
        }),
        layer: BoardLayer::BlEdgeCuts as i32,
        net: None,
        id: None,
        locked: 0,
    }
}

fn shape_on_layer(layer: BoardLayer, x0: f64, y0: f64, x1: f64, y1: f64) -> BoardGraphicShape {
    let mut shape = edge_cut_segment(x0, y0, x1, y1);
    shape.layer = layer as i32;
    shape
}

fn items_response(items: Vec<BoardGraphicShape>) -> GetItemsResponse {
    GetItemsResponse {
        header: None,
        status: ItemRequestStatus::IrsOk as i32,
        items: items
            .into_iter()
            .map(|s| DispatchTransport::pack(SHAPE_URL, &s))
            .collect(),
    }
}

fn nets(names: &[&str]) -> NetsResponse {
    NetsResponse {
        nets: names
            .iter()
            .map(|n| Net {
                code: None,
                name: n.to_string(),
            })
            .collect(),
    }
}

fn classes_for(pairs: &[(&str, &str)]) -> NetClassForNetsResponse {
    NetClassForNetsResponse {
        classes: pairs
            .iter()
            .map(|(net, class)| {
                (
                    net.to_string(),
                    NetClass {
                        name: class.to_string(),
                        ..NetClass::default()
                    },
                )
            })
            .collect(),
    }
}

/// Builder for a full diagnostics snapshot fixture.
struct SnapshotBuilder {
    transport: DispatchTransport,
}

impl SnapshotBuilder {
    fn new() -> Self {
        let transport = DispatchTransport::new().on(
            GET_BOARD_ENABLED_LAYERS_URL,
            DispatchTransport::pack(
                BOARD_ENABLED_LAYERS_RESPONSE_URL,
                &BoardEnabledLayersResponse {
                    copper_layer_count: 4,
                    layers: Vec::new(),
                },
            ),
        );
        Self { transport }
    }

    fn with_items(self, items: GetItemsResponse) -> Self {
        Self {
            transport: self.transport.on(
                GET_ITEMS_URL,
                DispatchTransport::pack(GET_ITEMS_RESPONSE_URL, &items),
            ),
        }
    }

    fn with_nets(self, nets: NetsResponse) -> Self {
        Self {
            transport: self.transport.on(
                GET_NETS_URL,
                DispatchTransport::pack(NETS_RESPONSE_URL, &nets),
            ),
        }
    }

    fn with_net_classes(self, classes: NetClassForNetsResponse) -> Self {
        Self {
            transport: self.transport.on(
                GET_NET_CLASS_FOR_NETS_URL,
                DispatchTransport::pack(NETCLASS_FOR_NETS_RESPONSE_URL, &classes),
            ),
        }
    }

    fn unhandled_items(self) -> Self {
        Self {
            transport: self.transport.unhandled(GET_ITEMS_URL),
        }
    }

    fn unhandled_nets(self) -> Self {
        Self {
            transport: self
                .transport
                .unhandled(GET_NETS_URL)
                .unhandled(GET_NET_CLASS_FOR_NETS_URL),
        }
    }

    /// Runs the snapshot, returning the diagnostics plus the recorded
    /// request type URLs.
    fn run(self) -> (BoardDiagnostics, Vec<String>) {
        let shared = SharedDispatch(Rc::new(RefCell::new(self.transport)));
        let mut client = KiCadClient::with_transport(
            Box::new(shared.clone()),
            Some("test"),
            2000,
        );
        let mut board = BoardHandle::new(&mut client, pcb_document());
        let d = get_board_diagnostics(&mut board).expect("get_board_diagnostics");
        let sent = shared.0.borrow().sent_type_urls.clone();
        (d, sent)
    }
}

// ---------------------------------------------------------------------------
// Net-class diagnostics
// ---------------------------------------------------------------------------

#[test]
fn net_classes_populated_from_effective_per_net_classes() {
    let (d, _) = SnapshotBuilder::new()
        .with_items(items_response(Vec::new()))
        .with_nets(nets(&["/A", "/B", "/C"]))
        .with_net_classes(classes_for(&[("/A", "Default"), ("/B", "Motor"), ("/C", "Motor")]))
        .run();
    assert_eq!(
        d.available_net_classes,
        vec!["Default".to_string(), "Motor".to_string()],
        "distinct effective classes, sorted"
    );
}

#[test]
fn composite_net_class_surfaces_constituents() {
    let mut classes = NetClassForNetsResponse::default();
    classes.classes.insert(
        "/A".to_string(),
        NetClass {
            name: String::new(), // composite/implicit class has no name
            constituents: vec!["Motor".to_string(), "Power".to_string()],
            ..NetClass::default()
        },
    );
    let (d, _) = SnapshotBuilder::new()
        .with_items(items_response(Vec::new()))
        .with_nets(nets(&["/A"]))
        .with_net_classes(classes)
        .run();
    assert_eq!(
        d.available_net_classes,
        vec!["Motor".to_string(), "Power".to_string()]
    );
}

#[test]
fn empty_board_no_net_classes_and_no_class_query() {
    let (d, sent) = SnapshotBuilder::new()
        .with_items(items_response(Vec::new()))
        .with_nets(nets(&[]))
        .run();
    assert!(d.available_net_classes.is_empty(), "empty board → no classes");
    assert!(
        !sent.iter().any(|url| url == GET_NET_CLASS_FOR_NETS_URL),
        "GetNetClassForNets must be skipped for a netless board, sent: {sent:?}"
    );
}

// ---------------------------------------------------------------------------
// Edge-cut bounding box
// ---------------------------------------------------------------------------

#[test]
fn edge_cut_bbox_populated_from_shapes() {
    let (d, _) = SnapshotBuilder::new()
        .with_items(items_response(vec![
            edge_cut_segment(0.0, 0.0, 40.0, 0.0),    // x: [0, 40]
            edge_cut_segment(40.0, 0.0, 40.0, 20.0),  // y: [0, 20]
            edge_cut_segment(-10.0, 20.0, 0.0, 20.0), // x_min: -10
            shape_on_layer(BoardLayer::BlFSilkS, 100.0, 100.0, 200.0, 200.0),
        ]))
        .with_nets(nets(&["/A"]))
        .with_net_classes(classes_for(&[("/A", "Default")]))
        .run();
    assert_eq!(d.copper_layer_count, 4);
    assert!((d.board_x_min_mm - (-10.0)).abs() < 1e-9);
    assert!((d.board_x_max_mm - 40.0).abs() < 1e-9);
    assert!((d.board_y_min_mm - 0.0).abs() < 1e-9);
    assert!((d.board_y_max_mm - 20.0).abs() < 1e-9);
    assert!((d.board_width_mm() - 50.0).abs() < 1e-9);
    assert!((d.board_height_mm() - 20.0).abs() < 1e-9);
}

#[test]
fn non_edge_cut_only_board_degrades_to_zero_bbox() {
    let (d, _) = SnapshotBuilder::new()
        .with_items(items_response(vec![shape_on_layer(
            BoardLayer::BlCmtsUser,
            5.0,
            5.0,
            6.0,
            6.0,
        )]))
        .run();
    assert_eq!(d.board_x_min_mm, 0.0);
    assert_eq!(d.board_x_max_mm, 0.0);
    assert_eq!(d.board_y_min_mm, 0.0);
    assert_eq!(d.board_y_max_mm, 0.0);
}

// ---------------------------------------------------------------------------
// Documented degradation path (per-field defaults on query failure)
// ---------------------------------------------------------------------------

#[test]
fn unhandled_shape_query_degrades_but_net_classes_stay_real() {
    let (d, _) = SnapshotBuilder::new()
        .unhandled_items()
        .with_nets(nets(&["/A"]))
        .with_net_classes(classes_for(&[("/A", "Motor")]))
        .run();
    assert_eq!(d.board_x_min_mm, 0.0, "bbox degrades to placeholder zeros");
    assert_eq!(d.board_x_max_mm, 0.0);
    assert_eq!(d.board_y_min_mm, 0.0);
    assert_eq!(d.board_y_max_mm, 0.0);
    assert_eq!(d.copper_layer_count, 4, "other fields stay real");
    assert_eq!(d.available_net_classes, vec!["Motor".to_string()]);
}

#[test]
fn unhandled_net_queries_degrade_but_bbox_stays_real() {
    let (d, _) = SnapshotBuilder::new()
        .with_items(items_response(vec![edge_cut_segment(-2.0, -1.0, 8.0, 4.0)]))
        .unhandled_nets()
        .run();
    assert!(d.available_net_classes.is_empty(), "classes degrade to empty");
    assert!((d.board_x_min_mm - (-2.0)).abs() < 1e-9, "bbox stays real");
    assert!((d.board_x_max_mm - 8.0).abs() < 1e-9);
    assert!((d.board_y_min_mm - (-1.0)).abs() < 1e-9);
    assert!((d.board_y_max_mm - 4.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// Request hygiene: the diagnostics flow sends exactly the documented commands
// ---------------------------------------------------------------------------

#[test]
fn diagnostics_flow_sends_exactly_the_documented_commands() {
    let (_, sent) = SnapshotBuilder::new()
        .with_items(items_response(Vec::new()))
        .with_nets(nets(&["/A"]))
        .with_net_classes(classes_for(&[("/A", "Default")]))
        .run();

    let mut expected = vec![
        GET_BOARD_ENABLED_LAYERS_URL,
        GET_ITEMS_URL,
        GET_NETS_URL,
        GET_NET_CLASS_FOR_NETS_URL,
    ];
    expected.sort_unstable();
    let mut actual = sent.clone();
    actual.sort_unstable();
    assert_eq!(actual, expected, "sent commands must match the documented set");

    // No command repeats, nothing else (writes, commits, interactive) leaks in.
    assert_eq!(sent.len(), expected.len(), "no duplicate commands");
}

// ---------------------------------------------------------------------------
// GetBoardOrigin — real query, no diagnostics field (documented in API.md)
// ---------------------------------------------------------------------------

#[test]
fn board_handle_get_board_origin_returns_origin_point() {
    let transport = DispatchTransport::new().on(
        GET_BOARD_ORIGIN_URL,
        DispatchTransport::pack(VECTOR2_URL, &Vector2 { x_nm: 1_500_000, y_nm: -2_500_000 }),
    );
    let mut client = KiCadClient::with_transport(Box::new(transport), Some("test"), 2000);
    let mut board = BoardHandle::new(&mut client, pcb_document());
    let origin = board
        .get_board_origin(BoardOriginType::BotDrill)
        .expect("get_board_origin");
    assert_eq!((origin.x_nm, origin.y_nm), (1_500_000, -2_500_000));
}

#[test]
fn board_handle_get_board_origin_failure_is_api_error() {
    let transport = DispatchTransport::new().unhandled(GET_BOARD_ORIGIN_URL);
    let mut client = KiCadClient::with_transport(Box::new(transport), Some("test"), 2000);
    let mut board = BoardHandle::new(&mut client, pcb_document());
    let err = board
        .get_board_origin(BoardOriginType::BotGrid)
        .expect_err("unhandled → error");
    assert!(matches!(err, KiCadError::Api { .. }));
}
