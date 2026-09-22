use ciclybog_router_core::Graph;
use std::{env, fs, io::BufWriter, io::Write};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Uso: export_graph <graph.bin> <output.geojson>");
        std::process::exit(2);
    }
    let graph = Graph::from_bytes(&fs::read(&args[1]).expect("Could not read the graph"))
        .expect("Invalid graph");
    let mut output = BufWriter::new(fs::File::create(&args[2]).expect("Could not create GeoJSON"));
    write!(output, "{{\"type\":\"FeatureCollection\",\"features\":[").unwrap();
    for (index, edge) in graph.edges.iter().enumerate() {
        if index > 0 {
            write!(output, ",").unwrap();
        }
        let feature = serde_json::json!({
            "type": "Feature",
            "properties": {
                "edge_index": index,
                "from": edge.from,
                "to": edge.to,
                "distance_meters": edge.distance_meters,
                "routing_cost": edge.routing_cost,
                "infrastructure": edge.infrastructure,
                "road_name": graph.string(edge.road_name),
                "bicycle_access": graph.string(edge.bicycle_access),
                "osm_way_id": edge.osm_way_id,
            },
            "geometry": {
                "type": "LineString",
                "coordinates": graph
                    .edge_geometry(edge)
                    .iter()
                    .map(|point| [point.lon, point.lat])
                    .collect::<Vec<_>>(),
            }
        });
        serde_json::to_writer(&mut output, &feature).unwrap();
    }
    writeln!(output, "]}}").unwrap();
    eprintln!("{} aristas exportadas a {}", graph.edges.len(), args[2]);
}
