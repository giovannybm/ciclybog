use ciclybog_router_core::{Coordinate, Infrastructure, PreparedGraph, RouteRequest};
use std::{env, fs, time::Instant};

fn main() {
    let path = env::args().nth(1).expect("Uso: inspect_graph <graph.bin>");
    let bytes = fs::read(&path).expect("No se pudo leer el grafo");
    let started = Instant::now();
    let prepared = PreparedGraph::from_bytes(&bytes).expect("Grafo inválido");
    let graph = prepared.graph();
    println!(
        "version={} size={:.1}MB load+index={:?} nodes={} edges={} geometries={} restrictions={} main_component={}",
        graph.version,
        bytes.len() as f64 / 1_000_000.0,
        started.elapsed(),
        graph.nodes.len(),
        graph.edges.len(),
        graph.geometries.len(),
        graph.turn_restrictions.len(),
        prepared.main_component_size()
    );
    let cycleway_edges = graph
        .edges
        .iter()
        .filter(|edge| edge.infrastructure == Infrastructure::Cycleway)
        .count();
    println!("cycleway_edges={cycleway_edges}");
    let point = |lon, lat| Coordinate { lon, lat };
    for (origin, destination) in [
        (point(-74.0721, 4.7110), point(-74.0480, 4.6760)),
        (point(-74.1400, 4.6300), point(-74.0550, 4.7500)),
        (point(-74.0600, 4.6500), point(-74.0620, 4.6520)),
    ] {
        let started = Instant::now();
        match prepared.route(&RouteRequest { origin, destination, alternatives: 1 }) {
            Ok(routes) => println!(
                "route {:?}: {:?}",
                started.elapsed(),
                routes
                    .iter()
                    .map(|route| format!(
                        "{:.0} m ({:.0}% cicloruta, {} segmentos)",
                        route.distance_meters,
                        100.0 * route.cycleway_meters / route.distance_meters,
                        route.segments.len()
                    ))
                    .collect::<Vec<_>>()
            ),
            Err(error) => println!("route {:?}: error {error}", started.elapsed()),
        }
    }
}
