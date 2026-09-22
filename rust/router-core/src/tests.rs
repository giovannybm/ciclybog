use super::*;

const STEP: f64 = 0.001; // ~111 m

fn point(x: f64, y: f64) -> Coordinate {
    Coordinate {
        lon: x * STEP,
        lat: y * STEP,
    }
}

struct Way {
    from: u32,
    to: u32,
    via: Vec<(f64, f64)>,
    two_way: bool,
    cost: f32,
    infrastructure: Infrastructure,
    name: &'static str,
    way_id: i64,
}

fn way(from: u32, to: u32, name: &'static str, way_id: i64) -> Way {
    Way {
        from,
        to,
        via: Vec::new(),
        two_way: true,
        cost: 1.0,
        infrastructure: Infrastructure::Conventional,
        name,
        way_id,
    }
}

fn build(nodes: &[(f64, f64)], ways: Vec<Way>, restrictions: Vec<TurnRestriction>) -> PreparedGraph {
    let mut builder = GraphBuilder::new();
    for &(x, y) in nodes {
        builder.add_node(point(x, y));
    }
    for way in ways {
        let mut geometry = vec![point(nodes[way.from as usize].0, nodes[way.from as usize].1)];
        geometry.extend(way.via.iter().map(|&(x, y)| point(x, y)));
        geometry.push(point(nodes[way.to as usize].0, nodes[way.to as usize].1));
        builder.add_segment(WaySegment {
            from: way.from,
            to: way.to,
            geometry,
            forward_cost: Some(way.cost),
            backward_cost: way.two_way.then_some(way.cost),
            infrastructure: way.infrastructure,
            road_name: Some(way.name),
            bicycle_access: None,
            osm_way_id: Some(way.way_id),
        });
    }
    for restriction in restrictions {
        builder.add_turn_restriction(restriction);
    }
    PreparedGraph::new(builder.build("test"))
}

fn request(from: (f64, f64), to: (f64, f64), alternatives: usize) -> RouteRequest {
    RouteRequest {
        origin: point(from.0, from.1),
        destination: point(to.0, to.1),
        alternatives,
    }
}

fn names(route: &Route) -> Vec<&str> {
    route
        .segments
        .iter()
        .filter_map(|segment| segment.road_name.as_deref())
        .collect()
}

#[test]
fn serializes_with_magic_header() {
    let graph = build(&[(0.0, 0.0), (1.0, 0.0)], vec![way(0, 1, "a", 1)], vec![]);
    let bytes = graph.graph().to_bytes().unwrap();
    assert_eq!(&bytes[..8], GRAPH_MAGIC);
    assert_eq!(Graph::from_bytes(&bytes).unwrap().version, "test");
    assert!(Graph::from_bytes(b"CICLYG01xxxx").is_err());
}

#[test]
fn prefers_cycleway_and_reports_cycleway_meters() {
    let mut cycleway = way(0, 1, "ciclovía", 1);
    cycleway.via = vec![(1.0, 0.3)];
    cycleway.infrastructure = Infrastructure::Cycleway;
    let mut avenue = way(0, 1, "avenida", 2);
    avenue.cost = 1.4;
    let graph = build(&[(0.0, 0.0), (2.0, 0.0)], vec![cycleway, avenue], vec![]);
    let routes = graph.route(&request((0.0, 0.0), (2.0, 0.0), 0)).unwrap();
    assert_eq!(names(&routes[0]), vec!["ciclovía"]);
    assert!((routes[0].cycleway_meters - routes[0].distance_meters).abs() < 0.01);
}

#[test]
fn falls_back_to_shortest_when_detour_is_too_long() {
    let mut cycleway = way(0, 1, "ciclovía", 1);
    cycleway.via = vec![(1.0, 1.5)];
    cycleway.infrastructure = Infrastructure::Cycleway;
    let mut avenue = way(0, 1, "avenida", 2);
    avenue.cost = 1.42;
    let graph = build(&[(0.0, 0.0), (2.0, 0.0)], vec![cycleway, avenue], vec![]);
    let routes = graph.route(&request((0.0, 0.0), (2.0, 0.0), 0)).unwrap();
    assert_eq!(names(&routes[0]), vec!["avenida"]);
}

#[test]
fn snaps_on_edges_and_splits_two_way_streets_without_detour() {
    // A(0,0) ─ B(4,0) two-way road; A ─ C(0,2) heading north.
    let graph = build(
        &[(0.0, 0.0), (4.0, 0.0), (0.0, 2.0)],
        vec![way(0, 1, "calle", 1), way(0, 2, "carrera", 2)],
        vec![],
    );
    let routes = graph.route(&request((1.0, 0.05), (0.0, 1.5), 0)).unwrap();
    let route = &routes[0];
    // One block toward A + 1.5 north; leaving toward B and returning would be ~8.5 steps.
    assert!(route.distance_meters < 2.7 * STEP * METERS_PER_DEGREE, "{}", route.distance_meters);
    let start = route.segments[0].geometry[0];
    assert!((start.lon - point(1.0, 0.0).lon).abs() < 1e-9 && start.lat.abs() < 1e-9);
    let end = *route.segments.last().unwrap().geometry.last().unwrap();
    assert!((end.lat - point(0.0, 1.5).lat).abs() < 1e-9);
}

#[test]
fn routes_directly_when_origin_and_destination_share_an_edge() {
    let graph = build(&[(0.0, 0.0), (4.0, 0.0)], vec![way(0, 1, "calle", 1)], vec![]);
    for (from, to) in [((1.0, 0.0), (3.0, 0.0)), ((3.0, 0.0), (1.0, 0.0))] {
        let routes = graph.route(&request(from, to, 0)).unwrap();
        assert_eq!(routes[0].segments.len(), 1);
        assert!((routes[0].distance_meters - 2.0 * STEP * METERS_PER_DEGREE).abs() < 5.0);
    }
}

#[test]
fn respects_one_way_on_shared_edge() {
    let mut street = way(0, 1, "calle", 1);
    street.two_way = false;
    let mut back = way(1, 0, "vuelta", 2);
    back.two_way = false;
    back.via = vec![(2.0, 1.0)];
    let graph = build(&[(0.0, 0.0), (4.0, 0.0)], vec![street, back], vec![]);
    let routes = graph.route(&request((3.0, 0.0), (1.0, 0.0), 0)).unwrap();
    assert!(names(&routes[0]).contains(&"vuelta"));
}

#[test]
fn rejects_points_far_from_the_network() {
    let graph = build(&[(0.0, 0.0), (4.0, 0.0)], vec![way(0, 1, "calle", 1)], vec![]);
    let origin_error = graph.route(&request((2.0, 5.0), (1.0, 0.0), 0)).unwrap_err();
    assert!(origin_error.contains("origin"), "{origin_error}");
    let destination_error = graph.route(&request((1.0, 0.0), (2.0, 5.0), 0)).unwrap_err();
    assert!(destination_error.contains("destination"), "{destination_error}");
}

/// S(0) ─calle 1─ V(1) ─calle 3─ T(2); S ─calle 2 (rodeo)─ V.
fn restriction_graph(restriction: TurnRestriction) -> PreparedGraph {
    let mut detour = way(0, 1, "rodeo", 2);
    detour.via = vec![(1.0, -1.0)];
    build(
        &[(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (4.0, 0.0)],
        vec![
            way(0, 1, "directa", 1),
            detour,
            way(1, 2, "salida", 3),
            way(1, 3, "recta", 4),
        ],
        vec![restriction],
    )
}

#[test]
fn uses_another_arrival_when_the_cheapest_one_forbids_the_turn() {
    let graph = restriction_graph(TurnRestriction {
        via_node: 1,
        from_way: 1,
        to_way: 3,
        only: false,
    });
    let routes = graph.route(&request((0.5, 0.0), (2.0, 1.5), 0)).unwrap();
    // The direct arrival at V cannot turn onto "exit": it backtracks to S and takes the detour.
    let route_names = names(&routes[0]);
    assert_eq!(route_names.last(), Some(&"salida"), "{route_names:?}");
    assert!(route_names.contains(&"rodeo"), "{route_names:?}");
}

#[test]
fn only_restrictions_allow_just_the_listed_way() {
    let graph = restriction_graph(TurnRestriction {
        via_node: 1,
        from_way: 1,
        to_way: 4,
        only: true,
    });
    let routes = graph.route(&request((0.5, 0.0), (2.0, 1.5), 0)).unwrap();
    assert!(names(&routes[0]).contains(&"rodeo"));
    let straight = graph.route(&request((0.5, 0.0), (3.5, 0.0), 0)).unwrap();
    assert!(names(&straight[0]).contains(&"directa"));
}

#[test]
fn no_u_turn_on_same_way_still_allows_going_straight() {
    let graph = build(
        &[(0.0, 0.0), (2.0, 0.0), (4.0, 0.0)],
        vec![way(0, 1, "avenida", 7), way(1, 2, "avenida", 7)],
        vec![TurnRestriction {
            via_node: 1,
            from_way: 7,
            to_way: 7,
            only: false,
        }],
    );
    assert!(graph.route(&request((0.5, 0.0), (3.5, 0.0), 0)).is_ok());
}

#[test]
fn ignores_one_way_traps_outside_the_main_component() {
    // Main network at y=0; one-way spur X(1,1)→Y(1,1.5) with no exit.
    let mut spur = way(3, 4, "trampa", 9);
    spur.two_way = false;
    let mut entry = way(1, 3, "entrada", 8);
    entry.two_way = false;
    let graph = build(
        &[(0.0, 0.0), (1.0, 0.0), (3.0, 0.0), (1.0, 1.0), (1.0, 1.5)],
        vec![way(0, 1, "calle", 1), way(1, 2, "calle", 1), entry, spur],
        vec![],
    );
    assert_eq!(graph.main_component_size(), 3);
    let routes = graph.route(&request((1.0, 1.4), (2.5, 0.0), 0)).unwrap();
    assert!(!names(&routes[0]).contains(&"trampa"));
}

#[test]
fn returns_a_distinct_alternative() {
    // Dos corredores paralelos de largo similar entre S(0,0) y T(4,0).
    let mut north = way(0, 1, "norte", 1);
    north.via = vec![(0.0, 1.0), (4.0, 1.0)];
    let mut south = way(0, 1, "sur", 2);
    south.via = vec![(0.0, -1.2), (4.0, -1.2)];
    let graph = build(&[(0.0, 0.0), (4.0, 0.0)], vec![north, south], vec![]);
    let routes = graph.route(&request((0.0, 0.0), (4.0, 0.0), 1)).unwrap();
    assert_eq!(routes.len(), 2);
    assert_ne!(names(&routes[0]), names(&routes[1]));
    assert!(routes[0].distance_meters <= routes[1].distance_meters);
    let single = graph.route(&request((0.0, 0.0), (4.0, 0.0), 0)).unwrap();
    assert_eq!(single.len(), 1);
}

#[test]
fn merges_consecutive_segments_with_same_attributes() {
    let graph = build(
        &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)],
        vec![way(0, 1, "avenida", 1), way(1, 2, "avenida", 1), way(2, 3, "avenida", 1)],
        vec![],
    );
    let routes = graph.route(&request((0.0, 0.0), (3.0, 0.0), 0)).unwrap();
    assert_eq!(routes[0].segments.len(), 1);
    assert_eq!(routes[0].segments[0].geometry.len(), 4);
}

/// Independent exhaustive search (Bellman-Ford over edge states).
fn brute_force_cost(graph: &PreparedGraph, query: &Query) -> f64 {
    let total = graph.graph.edges.len() + query.edges.len();
    let mut costs = vec![f64::INFINITY; total];
    let cost_of = |edge: u32| {
        graph.edge_distance(query, edge) * (graph.source_edge(query, edge).routing_cost as f64).max(1.0)
    };
    for edge in 0..total as u32 {
        if graph.edge_endpoints(query, edge).0 == query.origin {
            costs[edge as usize] = cost_of(edge);
        }
    }
    let mut changed = true;
    let mut neighbors = Vec::new();
    while changed {
        changed = false;
        for edge in 0..total as u32 {
            if !costs[edge as usize].is_finite() {
                continue;
            }
            let node = graph.edge_endpoints(query, edge).1;
            if node == query.destination {
                continue;
            }
            graph.outgoing_edges(query, node, &mut neighbors);
            for &next in &neighbors {
                let turn = turn_penalty(turn_angle(
                    graph.edge_bearings(query, edge).1,
                    graph.edge_bearings(query, next).0,
                ));
                let candidate = costs[edge as usize] + cost_of(next) + turn;
                if candidate + 1e-9 < costs[next as usize] {
                    costs[next as usize] = candidate;
                    changed = true;
                }
            }
        }
    }
    (0..total as u32)
        .filter(|&edge| graph.edge_endpoints(query, edge).1 == query.destination)
        .map(|edge| costs[edge as usize])
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn a_star_matches_exhaustive_search_on_random_grids() {
    let mut seed = 42u64;
    let mut random = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as f64 / (1u64 << 31) as f64
    };
    for _ in 0..25 {
        let size = 6;
        let mut nodes = Vec::new();
        for y in 0..size {
            for x in 0..size {
                nodes.push((x as f64 + random() * 0.4, y as f64 + random() * 0.4));
            }
        }
        let mut ways = Vec::new();
        let mut way_id = 0;
        for y in 0..size {
            for x in 0..size {
                let index = (y * size + x) as u32;
                for next in [(x + 1 < size).then(|| index + 1), (y + 1 < size).then(|| index + size as u32)]
                    .into_iter()
                    .flatten()
                {
                    way_id += 1;
                    let mut street = way(index, next, "calle", way_id);
                    street.cost = 1.0 + (random() * 0.6) as f32;
                    ways.push(street);
                }
            }
        }
        let graph = build(&nodes, ways, vec![]);
        let from = (random() * 5.0, random() * 5.0);
        let to = (random() * 5.0, random() * 5.0);
        let (Some(origin), Some(destination)) = (graph.snap(point(from.0, from.1)), graph.snap(point(to.0, to.1)))
        else {
            continue;
        };
        if haversine_distance(origin.point, destination.point) < 1.0 {
            continue;
        }
        let query = graph.build_query(origin, destination);
        let path = graph.search(&query, true, None).unwrap();
        let mut cost = 0.0;
        for (index, &edge) in path.edges.iter().enumerate() {
            cost += graph.edge_distance(&query, edge) * (graph.source_edge(&query, edge).routing_cost as f64).max(1.0);
            if index > 0 {
                cost += turn_penalty(turn_angle(
                    graph.edge_bearings(&query, path.edges[index - 1]).1,
                    graph.edge_bearings(&query, edge).0,
                ));
            }
        }
        let expected = brute_force_cost(&graph, &query);
        assert!((cost - expected).abs() < 1e-6, "A* {cost} vs exhaustivo {expected}");
    }
}
