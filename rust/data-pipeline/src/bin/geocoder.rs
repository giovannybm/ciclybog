use osmpbfreader::{OsmObj, OsmPbfReader, Tags};
use serde_json::json;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::{env, fs, io::BufReader, time::UNIX_EPOCH};

/// Etiquetas que convierten un elemento con nombre en un lugar buscable.
const PLACE_KEYS: [&str; 10] = [
    "amenity",
    "shop",
    "tourism",
    "leisure",
    "office",
    "historic",
    "public_transport",
    "railway",
    "healthcare",
    "place",
];
const STREET_SIMPLIFY_METERS: f64 = 2.0;
const METERS_PER_DEGREE_LON: f64 = 110_953.0;
const METERS_PER_DEGREE_LAT: f64 = 110_574.0;

type Point = (f64, f64);

fn tag<'a>(tags: &'a Tags, key: &str) -> Option<&'a str> {
    tags.get(key).map(|value| value.as_str())
}

fn useful_text(value: &str) -> bool {
    value.chars().filter(|character| character.is_alphabetic()).count() >= 2
}

fn place_kind(tags: &Tags) -> Option<String> {
    PLACE_KEYS
        .iter()
        .find_map(|key| tag(tags, key).map(|value| format!("{key}={value}")))
        .or_else(|| tag(tags, "building").map(|_| "building".to_string()))
}

fn round(value: f64) -> f64 {
    (value * 100_000.0).round() / 100_000.0
}

fn centroid(points: &[Point]) -> Point {
    let count = points.len() as f64;
    let (lon, lat) = points
        .iter()
        .fold((0.0, 0.0), |sum, point| (sum.0 + point.0, sum.1 + point.1));
    (lon / count, lat / count)
}

/// Douglas-Peucker en metros locales; conserva la forma de la vía para cruzarla con otras.
fn simplify(points: &[Point], tolerance: f64) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let project = |point: Point| (point.0 * METERS_PER_DEGREE_LON, point.1 * METERS_PER_DEGREE_LAT);
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0, points.len() - 1)];
    while let Some((start, end)) = stack.pop() {
        let (a, b) = (project(points[start]), project(points[end]));
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length_squared = dx * dx + dy * dy;
        let mut farthest = (0.0, 0);
        for (index, point) in points.iter().enumerate().take(end).skip(start + 1) {
            let p = project(*point);
            let t = if length_squared == 0.0 {
                0.0
            } else {
                (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length_squared).clamp(0.0, 1.0)
            };
            let distance = ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt();
            if distance > farthest.0 {
                farthest = (distance, index);
            }
        }
        if farthest.0 > tolerance {
            keep[farthest.1] = true;
            stack.push((start, farthest.1));
            stack.push((farthest.1, end));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter(|(_, keep)| *keep)
        .map(|(point, _)| *point)
        .collect()
}

fn collect(
    tags: &Tags,
    point: Point,
    places: &mut Vec<(String, String, Point)>,
    addresses: &mut Vec<(String, String, Point)>,
) {
    if let (Some(street), Some(number)) = (tag(tags, "addr:street"), tag(tags, "addr:housenumber")) {
        if useful_text(street) && number.chars().any(|character| character.is_ascii_digit()) {
            addresses.push((street.trim().to_string(), number.trim().to_string(), point));
        }
    }
    if let (Some(name), Some(kind)) = (tag(tags, "name"), place_kind(tags)) {
        if useful_text(name) {
            places.push((name.trim().to_string(), kind, point));
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Uso: geocoder <input.osm.pbf> <output.json>");
        std::process::exit(2);
    }
    let file = fs::File::open(&args[1]).expect("No se pudo abrir el PBF");
    let source_modified = file
        .metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    let mut reader = OsmPbfReader::new(BufReader::new(file));
    let mut coordinates = HashMap::<i64, Point>::new();
    let mut streets = BTreeMap::<String, Vec<Vec<f64>>>::new();
    let mut places = Vec::new();
    let mut addresses = Vec::new();

    for object in reader.iter() {
        match object.expect("PBF inválido") {
            OsmObj::Node(node) => {
                let point = (node.lon(), node.lat());
                coordinates.insert(node.id.0, point);
                collect(&node.tags, point, &mut places, &mut addresses);
            }
            OsmObj::Way(way) => {
                let points: Vec<Point> = way
                    .nodes
                    .iter()
                    .filter_map(|id| coordinates.get(&id.0).copied())
                    .collect();
                if points.is_empty() {
                    continue;
                }
                if let Some(highway) = tag(&way.tags, "highway") {
                    let name = tag(&way.tags, "name").filter(|name| useful_text(name));
                    if let Some(name) = name.filter(|_| {
                        points.len() >= 2 && !matches!(highway, "platform" | "elevator" | "services" | "rest_area")
                    }) {
                        let line = simplify(&points, STREET_SIMPLIFY_METERS)
                            .into_iter()
                            .flat_map(|(lon, lat)| [round(lon), round(lat)])
                            .collect();
                        streets.entry(name.trim().to_string()).or_default().push(line);
                    }
                    continue;
                }
                collect(&way.tags, centroid(&points), &mut places, &mut addresses);
            }
            // Las relaciones requieren ensamblar miembros; sus lugares suelen existir también como nodo o vía.
            OsmObj::Relation(_) => {}
        }
    }

    let mut seen_places = HashSet::new();
    places.retain(|(name, _, point)| {
        seen_places.insert((
            name.to_lowercase(),
            (point.0 * 1000.0).round() as i64,
            (point.1 * 1000.0).round() as i64,
        ))
    });
    let mut seen_addresses = HashSet::new();
    addresses.retain(|(street, number, _)| seen_addresses.insert((street.to_lowercase(), number.to_lowercase())));

    let street_count = streets.len();
    let output = json!({
        "version": 2,
        "source": "© OpenStreetMap contributors (ODbL)",
        "input": args[1],
        "input_modified_unix": source_modified,
        "streets": streets
            .into_iter()
            .map(|(name, lines)| json!({ "name": name, "lines": lines }))
            .collect::<Vec<_>>(),
        "places": places
            .iter()
            .map(|(name, kind, point)| json!([name, kind, round(point.0), round(point.1)]))
            .collect::<Vec<_>>(),
        "addresses": addresses
            .iter()
            .map(|(street, number, point)| json!([street, number, round(point.0), round(point.1)]))
            .collect::<Vec<_>>(),
    });
    let bytes = serde_json::to_vec(&output).expect("No se pudo serializar el índice");
    fs::write(&args[2], &bytes).expect("No se pudo escribir el índice");
    println!(
        "Índice geocoder v2: {street_count} vías con nombre, {} lugares, {} direcciones, {:.1} MB",
        places.len(),
        addresses.len(),
        bytes.len() as f64 / 1_000_000.0
    );
}
