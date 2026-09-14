use ciclybog_router_core::{Coordinate, GraphBuilder, Infrastructure, TurnRestriction, WaySegment};
use osmpbfreader::{OsmId, OsmObj, OsmPbfReader, Tags};
use serde::Deserialize;
use std::{collections::HashMap, env, fs, io::BufReader, process};

const GRAPH_VERSION: &str = "osm-pbf-v4-contracted";
// El GeoJSON de muestra puede cortar la misma vía con pequeñas diferencias
// numéricas. Un umbral de ~22 m conecta esos extremos sin exigir coincidencia exacta.
const SNAP_TOLERANCE_DEGREES: f64 = 0.0002;
const SNAP_CELL_SCALE: f64 = 10_000.0;
const DISMOUNT_FACTOR: f32 = 3.0;
const CYCLEWAY_SIDE_KEYS: [&str; 4] = ["cycleway", "cycleway:both", "cycleway:left", "cycleway:right"];

#[derive(Deserialize)]
struct FeatureCollection {
    features: Vec<Feature>,
}
#[derive(Deserialize)]
struct Feature {
    geometry: Geometry,
    properties: Properties,
}
#[derive(Deserialize)]
struct Geometry {
    coordinates: Vec<Vec<f64>>,
}
#[derive(Default, Deserialize)]
struct Properties {
    #[serde(default)]
    forward_cost: Option<f32>,
    #[serde(default)]
    backward_cost: Option<f32>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    infrastructure: Option<String>,
    #[serde(default)]
    bicycle_access: Option<String>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if !(3..=4).contains(&args.len()) {
        eprintln!("Uso: ciclybog-data-pipeline <network.geojson|bogota.osm.pbf> <output.bin> [cycleways.geojson]");
        process::exit(2);
    }
    if args[1].ends_with(".pbf") {
        build_from_osm_pbf(&args[1], &args[2], args.get(3).map(String::as_str));
    } else {
        build_from_geojson(&args[1], &args[2]);
    }
}

fn build_from_geojson(input_path: &str, output_path: &str) {
    let input: FeatureCollection =
        serde_json::from_str(&fs::read_to_string(input_path).expect("No se pudo leer el GeoJSON"))
            .expect("GeoJSON inválido");
    let mut builder = GraphBuilder::new();
    let mut nodes = Vec::<Coordinate>::new();
    let mut cells = HashMap::<(i64, i64), Vec<u32>>::new();
    let mut node_id = |coordinate: Coordinate, builder: &mut GraphBuilder| {
        let cell = (
            (coordinate.lon * SNAP_CELL_SCALE).round() as i64,
            (coordinate.lat * SNAP_CELL_SCALE).round() as i64,
        );
        for lon_cell in (cell.0 - 1)..=(cell.0 + 1) {
            for lat_cell in (cell.1 - 1)..=(cell.1 + 1) {
                let found = cells.get(&(lon_cell, lat_cell)).and_then(|candidates| {
                    candidates.iter().copied().find(|&id| {
                        let existing = nodes[id as usize];
                        (existing.lon - coordinate.lon).powi(2) + (existing.lat - coordinate.lat).powi(2)
                            <= SNAP_TOLERANCE_DEGREES.powi(2)
                    })
                });
                if let Some(id) = found {
                    return id;
                }
            }
        }
        let id = builder.add_node(coordinate);
        nodes.push(coordinate);
        cells.entry(cell).or_default().push(id);
        id
    };
    for feature in input.features {
        if feature.geometry.coordinates.len() < 2 {
            continue;
        }
        let geometry: Vec<Coordinate> = feature
            .geometry
            .coordinates
            .iter()
            .map(|point| Coordinate { lon: point[0], lat: point[1] })
            .collect();
        let from = node_id(geometry[0], &mut builder);
        let to = node_id(*geometry.last().unwrap(), &mut builder);
        let infrastructure = match feature.properties.infrastructure.as_deref() {
            Some("cycleway") => Infrastructure::Cycleway,
            Some("conventional") => Infrastructure::Conventional,
            _ => Infrastructure::Unknown,
        };
        builder.add_segment(WaySegment {
            from,
            to,
            geometry,
            forward_cost: feature.properties.forward_cost,
            backward_cost: feature.properties.backward_cost,
            infrastructure,
            road_name: feature.properties.name.as_deref(),
            bicycle_access: feature.properties.bicycle_access.as_deref(),
            osm_way_id: None,
        });
    }
    let graph = builder.build("geojson-sample-v4");
    fs::write(output_path, graph.to_bytes().expect("No se pudo serializar el grafo"))
        .expect("No se pudo escribir el grafo");
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum OneWay {
    Forward,
    Reverse,
    Both,
}

#[derive(Debug, PartialEq)]
struct WayProfile {
    infrastructure: Infrastructure,
    factor: f32,
    bicycle_access: Option<String>,
    oneway: OneWay,
}

struct AcceptedWay {
    id: i64,
    name: Option<String>,
    profile: WayProfile,
    /// Tramos continuos de nodos con coordenadas conocidas.
    pieces: Vec<Vec<i64>>,
}

fn build_from_osm_pbf(input_path: &str, output_path: &str, cycleways_output: Option<&str>) {
    let file = fs::File::open(input_path).expect("No se pudo abrir el OSM PBF");
    let mut reader = OsmPbfReader::new(BufReader::new(file));
    let mut coordinates = HashMap::new();
    let mut raw_ways = Vec::new();
    let mut restrictions = Vec::new();
    let mut ignored_restrictions = 0usize;

    for object in reader.iter() {
        match object.expect("OSM PBF inválido") {
            OsmObj::Node(node) => {
                coordinates.insert(
                    node.id.0,
                    Coordinate {
                        lon: node.decimicro_lon as f64 / 10_000_000.0,
                        lat: node.decimicro_lat as f64 / 10_000_000.0,
                    },
                );
            }
            OsmObj::Way(way) => {
                if let Some(profile) = classify_way(&way.tags) {
                    raw_ways.push((way, profile));
                }
            }
            OsmObj::Relation(relation) => {
                if tag(&relation.tags, "type") != Some("restriction") {
                    continue;
                }
                let member = |role: &str| relation.refs.iter().find(|reference| reference.role == role);
                let way_member = |role: &str| match member(role).map(|reference| reference.member) {
                    Some(OsmId::Way(id)) => Some(id.0),
                    _ => None,
                };
                let via = match member("via").map(|reference| reference.member) {
                    Some(OsmId::Node(id)) => Some(id.0),
                    _ => None,
                };
                match (bicycle_restriction(&relation.tags), way_member("from"), way_member("to"), via) {
                    (Some(only), Some(from_way), Some(to_way), Some(via)) => {
                        restrictions.push((via, from_way, to_way, only))
                    }
                    _ => ignored_restrictions += 1,
                }
            }
        }
    }

    let mut skipped_nodes = 0usize;
    let mut cycleway_features = Vec::new();
    let mut usage = HashMap::<i64, u32>::new();
    let ways: Vec<AcceptedWay> = raw_ways
        .into_iter()
        .map(|(way, profile)| {
            let mut pieces = vec![Vec::new()];
            for node in &way.nodes {
                if !coordinates.contains_key(&node.0) {
                    skipped_nodes += 1;
                    pieces.push(Vec::new());
                    continue;
                }
                let piece = pieces.last_mut().unwrap();
                if piece.last() != Some(&node.0) {
                    piece.push(node.0);
                }
            }
            pieces.retain(|piece| piece.len() >= 2);
            if profile.infrastructure == Infrastructure::Cycleway {
                for piece in &pieces {
                    cycleway_features.push(serde_json::json!({
                        "type": "Feature",
                        "properties": {
                            "name": tag(&way.tags, "name"),
                            "highway": tag(&way.tags, "highway"),
                            "bicycle": profile.bicycle_access,
                            "oneway": tag(&way.tags, "oneway")
                        },
                        "geometry": {
                            "type": "LineString",
                            "coordinates": piece.iter().map(|node| {
                                let point = coordinates[node];
                                [point.lon, point.lat]
                            }).collect::<Vec<_>>()
                        }
                    }));
                }
            }
            AcceptedWay {
                id: way.id.0,
                name: tag(&way.tags, "name").map(str::to_string),
                profile,
                pieces,
            }
        })
        .collect();

    // Un nodo es de decisión si lo usan varias vías, es extremo de tramo o es vía de una restricción.
    for way in &ways {
        for piece in &way.pieces {
            for node in piece {
                *usage.entry(*node).or_default() += 1;
            }
            *usage.entry(piece[0]).or_default() += 1;
            *usage.entry(*piece.last().unwrap()).or_default() += 1;
        }
    }
    for (via, ..) in &restrictions {
        if let Some(count) = usage.get_mut(via) {
            *count += 2;
        }
    }

    let mut builder = GraphBuilder::new();
    let mut node_ids = HashMap::<i64, u32>::new();
    let mut graph_node = |osm_id: i64, builder: &mut GraphBuilder| {
        *node_ids
            .entry(osm_id)
            .or_insert_with(|| builder.add_node(coordinates[&osm_id]))
    };
    for way in &ways {
        let (forward_cost, backward_cost) = match way.profile.oneway {
            OneWay::Forward => (Some(way.profile.factor), None),
            OneWay::Reverse => (None, Some(way.profile.factor)),
            OneWay::Both => (Some(way.profile.factor), Some(way.profile.factor)),
        };
        for piece in &way.pieces {
            let mut start = 0;
            for index in 1..piece.len() {
                if index < piece.len() - 1 && usage[&piece[index]] < 2 {
                    continue;
                }
                let from = graph_node(piece[start], &mut builder);
                let to = graph_node(piece[index], &mut builder);
                builder.add_segment(WaySegment {
                    from,
                    to,
                    geometry: piece[start..=index].iter().map(|node| coordinates[node]).collect(),
                    forward_cost,
                    backward_cost,
                    infrastructure: way.profile.infrastructure,
                    road_name: way.name.as_deref(),
                    bicycle_access: way.profile.bicycle_access.as_deref(),
                    osm_way_id: Some(way.id),
                });
                start = index;
            }
        }
    }
    let kept_restrictions = restrictions
        .iter()
        .filter_map(|(via, from_way, to_way, only)| {
            node_ids.get(via).map(|&via_node| TurnRestriction {
                via_node,
                from_way: *from_way,
                to_way: *to_way,
                only: *only,
            })
        })
        .collect::<Vec<_>>();
    let kept_restriction_count = kept_restrictions.len();
    for restriction in kept_restrictions {
        builder.add_turn_restriction(restriction);
    }
    let graph = builder.build(GRAPH_VERSION);
    let bytes = graph.to_bytes().expect("No se pudo serializar el grafo OSM");
    fs::write(output_path, &bytes).expect("No se pudo escribir el grafo OSM");
    eprintln!(
        "Grafo OSM {GRAPH_VERSION}: {} vías, {} nodos, {} aristas, {} geometrías, {} restricciones ({} ignoradas para bicicleta o sin nodo vía), {} nodos sin coordenadas, {:.1} MB",
        ways.len(),
        graph.nodes.len(),
        graph.edges.len(),
        graph.geometries.len(),
        kept_restriction_count,
        ignored_restrictions,
        skipped_nodes,
        bytes.len() as f64 / 1_000_000.0
    );
    if let Some(cycleways_output) = cycleways_output {
        let collection = serde_json::json!({ "type": "FeatureCollection", "features": cycleway_features });
        fs::write(
            cycleways_output,
            serde_json::to_vec(&collection).expect("No se pudo serializar la capa ciclista"),
        )
        .expect("No se pudo escribir la capa ciclista");
        eprintln!("Capa ciclista OSM guardada en: {cycleways_output}");
    }
}

fn tag<'a>(tags: &'a Tags, key: &str) -> Option<&'a str> {
    tags.get(key).map(|value| value.as_str())
}

/// Devuelve `Some(only)` si la relación de restricción aplica a bicicletas.
fn bicycle_restriction(tags: &Tags) -> Option<bool> {
    if tag(tags, "type") != Some("restriction") {
        return None;
    }
    let excepts_bicycle = tag(tags, "except")
        .map(|value| value.split(';').any(|vehicle| vehicle.trim() == "bicycle"))
        .unwrap_or(false);
    if excepts_bicycle {
        return None;
    }
    // `restriction:motorcar`, `restriction:hgv`, etc. no aplican a bicicletas.
    let value = tag(tags, "restriction:bicycle").or_else(|| tag(tags, "restriction"))?;
    if value.starts_with("only_") {
        Some(true)
    } else if value.starts_with("no_") {
        Some(false)
    } else {
        None
    }
}

fn classify_way(tags: &Tags) -> Option<WayProfile> {
    let highway = tag(tags, "highway")?;
    let allowed = matches!(
        highway,
        "cycleway"
            | "path"
            | "track"
            | "footway"
            | "pedestrian"
            | "living_street"
            | "residential"
            | "service"
            | "unclassified"
            | "alley"
            | "tertiary"
            | "tertiary_link"
            | "secondary"
            | "secondary_link"
            | "primary"
            | "primary_link"
    );
    if !allowed {
        return None;
    }
    let bicycle = tag(tags, "bicycle");
    if matches!(bicycle, Some("no" | "private")) {
        return None;
    }
    let dismount = bicycle == Some("dismount");
    let bicycle_accessible = matches!(bicycle, Some("yes" | "designated" | "permissive" | "official"));
    // Un permiso ciclista explícito prevalece sobre restricciones generales.
    let generally_closed = ["access", "vehicle"]
        .iter()
        .any(|key| matches!(tag(tags, key), Some("no" | "private")));
    if generally_closed && !bicycle_accessible && !dismount {
        return None;
    }
    let has_cycleway_tag = CYCLEWAY_SIDE_KEYS.iter().any(|key| {
        matches!(
            tag(tags, key),
            Some("track" | "lane" | "opposite_track" | "opposite_lane")
        )
    });
    if matches!(highway, "footway" | "pedestrian") && !bicycle_accessible && !has_cycleway_tag && !dismount {
        return None;
    }
    let is_cycleway = !dismount
        && (highway == "cycleway"
            || has_cycleway_tag
            || (matches!(highway, "path" | "track" | "footway" | "pedestrian") && bicycle_accessible));
    let factor = if dismount {
        DISMOUNT_FACTOR
    } else {
        match highway {
            "cycleway" => 1.0,
            "path" | "track" if is_cycleway => 1.08,
            "path" => 1.30,
            "track" => 1.22,
            "footway" | "pedestrian" => 1.18,
            "living_street" | "residential" | "service" | "unclassified" | "alley" => 1.05,
            "tertiary" | "tertiary_link" => 1.15,
            "secondary" | "secondary_link" => 1.28,
            "primary" | "primary_link" => 1.42,
            _ => 1.0,
        }
    };
    Some(WayProfile {
        infrastructure: if is_cycleway {
            Infrastructure::Cycleway
        } else {
            Infrastructure::Conventional
        },
        factor,
        bicycle_access: bicycle.map(str::to_string),
        oneway: oneway_direction(tags),
    })
}

fn oneway_direction(tags: &Tags) -> OneWay {
    match tag(tags, "oneway:bicycle") {
        Some("no") => return OneWay::Both,
        Some("yes" | "true" | "1") => return OneWay::Forward,
        Some("-1") => return OneWay::Reverse,
        _ => {}
    }
    let base = if tag(tags, "junction") == Some("roundabout") {
        OneWay::Forward
    } else {
        match tag(tags, "oneway") {
            Some("yes" | "true" | "1") => OneWay::Forward,
            Some("-1") => OneWay::Reverse,
            _ => OneWay::Both,
        }
    };
    let contraflow = CYCLEWAY_SIDE_KEYS
        .iter()
        .any(|key| tag(tags, key).map(|value| value.starts_with("opposite")).unwrap_or(false))
        || ["cycleway:left:oneway", "cycleway:right:oneway", "cycleway:both:oneway"]
            .iter()
            .any(|key| matches!(tag(tags, key), Some("no" | "-1")));
    if contraflow {
        OneWay::Both
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(pairs: &[(&str, &str)]) -> Tags {
        let mut tags = Tags::new();
        for (key, value) in pairs {
            tags.insert((*key).into(), (*value).into());
        }
        tags
    }

    #[test]
    fn cycleway_no_does_not_mark_cycle_infrastructure() {
        let profile = classify_way(&tags(&[("highway", "secondary"), ("cycleway:both", "no")])).unwrap();
        assert_eq!(profile.infrastructure, Infrastructure::Conventional);
        let profile = classify_way(&tags(&[("highway", "residential"), ("cycleway:right", "separate")])).unwrap();
        assert_eq!(profile.infrastructure, Infrastructure::Conventional);
        let profile = classify_way(&tags(&[("highway", "residential"), ("cycleway:left:width", "1.5")])).unwrap();
        assert_eq!(profile.infrastructure, Infrastructure::Conventional);
        let profile = classify_way(&tags(&[("highway", "primary"), ("cycleway:right", "track")])).unwrap();
        assert_eq!(profile.infrastructure, Infrastructure::Cycleway);
    }

    #[test]
    fn explicit_bicycle_access_overrides_general_access() {
        assert!(classify_way(&tags(&[("highway", "service"), ("access", "no")])).is_none());
        assert!(classify_way(&tags(&[("highway", "service"), ("access", "no"), ("bicycle", "yes")])).is_some());
        assert!(classify_way(&tags(&[("highway", "residential"), ("vehicle", "private")])).is_none());
        assert!(classify_way(&tags(&[("highway", "residential"), ("bicycle", "no")])).is_none());
        assert!(classify_way(&tags(&[("highway", "footway")])).is_none());
    }

    #[test]
    fn dismount_is_routable_but_expensive() {
        let profile = classify_way(&tags(&[("highway", "footway"), ("bicycle", "dismount")])).unwrap();
        assert_eq!(profile.infrastructure, Infrastructure::Conventional);
        assert_eq!(profile.factor, DISMOUNT_FACTOR);
    }

    #[test]
    fn contraflow_for_bicycles_is_two_way() {
        assert_eq!(oneway_direction(&tags(&[("oneway", "yes")])), OneWay::Forward);
        assert_eq!(oneway_direction(&tags(&[("oneway", "yes"), ("oneway:bicycle", "no")])), OneWay::Both);
        assert_eq!(oneway_direction(&tags(&[("oneway", "yes"), ("cycleway", "opposite_lane")])), OneWay::Both);
        assert_eq!(oneway_direction(&tags(&[("oneway", "-1"), ("cycleway:left", "opposite")])), OneWay::Both);
        assert_eq!(oneway_direction(&tags(&[("junction", "roundabout")])), OneWay::Forward);
        assert_eq!(oneway_direction(&tags(&[("oneway:bicycle", "yes")])), OneWay::Forward);
    }

    #[test]
    fn restrictions_are_filtered_for_bicycles() {
        let base = [("type", "restriction"), ("restriction", "no_left_turn")];
        assert_eq!(bicycle_restriction(&tags(&base)), Some(false));
        assert_eq!(
            bicycle_restriction(&tags(&[base[0], base[1], ("except", "psv;bicycle")])),
            None
        );
        assert_eq!(
            bicycle_restriction(&tags(&[("type", "restriction"), ("restriction:motorcar", "no_u_turn")])),
            None
        );
        assert_eq!(
            bicycle_restriction(&tags(&[base[0], base[1], ("restriction:bicycle", "only_straight_on")])),
            Some(true)
        );
    }
}
