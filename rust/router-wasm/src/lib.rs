use ciclybog_router_core::{Coordinate, PreparedGraph, RouteRequest};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Router {
    graph: PreparedGraph,
}

#[wasm_bindgen]
impl Router {
    /// Deserializes the graph and builds routing indexes once.
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Result<Router, JsValue> {
        PreparedGraph::from_bytes(bytes)
            .map(|graph| Router { graph })
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn version(&self) -> String {
        self.graph.graph().version.clone()
    }

    pub fn route(
        &self,
        origin_lon: f64,
        origin_lat: f64,
        destination_lon: f64,
        destination_lat: f64,
        alternatives: usize,
    ) -> Result<JsValue, JsValue> {
        let request = RouteRequest {
            origin: Coordinate {
                lon: origin_lon,
                lat: origin_lat,
            },
            destination: Coordinate {
                lon: destination_lon,
                lat: destination_lat,
            },
            alternatives,
        };
        let routes = self
            .graph
            .route(&request)
            .map_err(|error| JsValue::from_str(&error))?;
        serde_wasm_bindgen::to_value(&routes).map_err(|error| JsValue::from_str(&error.to_string()))
    }
}
