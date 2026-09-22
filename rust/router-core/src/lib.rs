use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

pub const GRAPH_MAGIC: &[u8; 8] = b"CICLYG02";
/// Maximum distance between the selected point and the road it snaps to.
pub const MAX_SNAP_METERS: f64 = 250.0;
const MAX_PREFERRED_DETOUR_RATIO: f64 = 1.18;
const MAX_ALTERNATIVE_DISTANCE_RATIO: f64 = 1.4;
const MAX_ALTERNATIVE_OVERLAP: f64 = 0.8;
const ALTERNATIVE_PENALTIES: [f32; 2] = [1.6, 2.5];
const GRID_CELL_DEGREES: f64 = 0.001;
const METERS_PER_DEGREE: f64 = 111_320.0;
const NO_EDGE: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct Coordinate {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Infrastructure {
    Cycleway,
    Conventional,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Node {
    pub coordinate: Coordinate,
}

/// Directed edge. Both directions of a two-way road share `geometry`; `reversed`
/// indicates whether it is traversed opposite to the stored direction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Edge {
    pub from: u32,
    pub to: u32,
    pub distance_meters: f32,
    pub routing_cost: f32,
    pub geometry: u32,
    pub reversed: bool,
    pub infrastructure: Infrastructure,
    pub road_name: Option<u32>,
    pub bicycle_access: Option<u32>,
    pub osm_way_id: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TurnRestriction {
    /// Internal index of the turn node.
    pub via_node: u32,
    pub from_way: i64,
    pub to_way: i64,
    pub only: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Graph {
    pub version: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub geometries: Vec<Vec<Coordinate>>,
    pub strings: Vec<String>,
    pub outgoing_offsets: Vec<u32>,
    pub outgoing_edges: Vec<u32>,
    pub incoming_offsets: Vec<u32>,
    pub incoming_edges: Vec<u32>,
    pub turn_restrictions: Vec<TurnRestriction>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct RouteSegment {
    pub geometry: Vec<Coordinate>,
    pub distance_meters: f64,
    pub infrastructure: Infrastructure,
    pub road_name: Option<String>,
    pub bicycle_access: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Route {
    pub distance_meters: f64,
    pub cycleway_meters: f64,
    pub segments: Vec<RouteSegment>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RouteRequest {
    pub origin: Coordinate,
    pub destination: Coordinate,
    pub alternatives: usize,
}

impl Graph {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        let mut output = GRAPH_MAGIC.to_vec();
        output.extend(bincode::serialize(self)?);
        Ok(output)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < GRAPH_MAGIC.len() || &bytes[..GRAPH_MAGIC.len()] != GRAPH_MAGIC {
            return Err("Invalid or outdated graph format; regenerate bogota-graph.bin".into());
        }
        bincode::deserialize(&bytes[GRAPH_MAGIC.len()..]).map_err(|error| error.to_string())
    }

    /// Edge geometry in its traversal direction.
    pub fn edge_geometry(&self, edge: &Edge) -> Vec<Coordinate> {
        let mut geometry = self.geometries[edge.geometry as usize].clone();
        if edge.reversed {
            geometry.reverse();
        }
        geometry
    }

    pub fn string(&self, index: Option<u32>) -> Option<&str> {
        index.and_then(|index| self.strings.get(index as usize).map(String::as_str))
    }

    fn outgoing(&self, node: u32) -> &[u32] {
        let start = self.outgoing_offsets[node as usize] as usize;
        let end = self.outgoing_offsets[node as usize + 1] as usize;
        &self.outgoing_edges[start..end]
    }

    fn incoming(&self, node: u32) -> &[u32] {
        let start = self.incoming_offsets[node as usize] as usize;
        let end = self.incoming_offsets[node as usize + 1] as usize;
        &self.incoming_edges[start..end]
    }
}

/// Road segment between two decision nodes, used by the pipeline and tests.
pub struct WaySegment<'a> {
    pub from: u32,
    pub to: u32,
    pub geometry: Vec<Coordinate>,
    pub forward_cost: Option<f32>,
    pub backward_cost: Option<f32>,
    pub infrastructure: Infrastructure,
    pub road_name: Option<&'a str>,
    pub bicycle_access: Option<&'a str>,
    pub osm_way_id: Option<i64>,
}

#[derive(Default)]
pub struct GraphBuilder {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    geometries: Vec<Vec<Coordinate>>,
    strings: Vec<String>,
    string_ids: HashMap<String, u32>,
    turn_restrictions: Vec<TurnRestriction>,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, coordinate: Coordinate) -> u32 {
        self.nodes.push(Node { coordinate });
        (self.nodes.len() - 1) as u32
    }

    fn intern(&mut self, value: Option<&str>) -> Option<u32> {
        let value = value?;
        if let Some(&id) = self.string_ids.get(value) {
            return Some(id);
        }
        let id = self.strings.len() as u32;
        self.strings.push(value.to_string());
        self.string_ids.insert(value.to_string(), id);
        Some(id)
    }

    pub fn add_segment(&mut self, segment: WaySegment) {
        if segment.geometry.len() < 2
            || (segment.forward_cost.is_none() && segment.backward_cost.is_none())
        {
            return;
        }
        let distance_meters = geometry_distance(&segment.geometry) as f32;
        if distance_meters <= 0.0 {
            return;
        }
        let geometry = self.geometries.len() as u32;
        self.geometries.push(segment.geometry);
        let road_name = self.intern(segment.road_name);
        let bicycle_access = self.intern(segment.bicycle_access);
        let mut push = |from, to, reversed, routing_cost| {
            self.edges.push(Edge {
                from,
                to,
                distance_meters,
                routing_cost,
                geometry,
                reversed,
                infrastructure: segment.infrastructure,
                road_name,
                bicycle_access,
                osm_way_id: segment.osm_way_id,
            })
        };
        if let Some(cost) = segment.forward_cost {
            push(segment.from, segment.to, false, cost);
        }
        if let Some(cost) = segment.backward_cost {
            push(segment.to, segment.from, true, cost);
        }
    }

    pub fn add_turn_restriction(&mut self, restriction: TurnRestriction) {
        self.turn_restrictions.push(restriction);
    }

    pub fn build(self, version: &str) -> Graph {
        let (outgoing_offsets, outgoing_edges, incoming_offsets, incoming_edges) =
            build_csr(&self.edges, self.nodes.len());
        Graph {
            version: version.into(),
            nodes: self.nodes,
            edges: self.edges,
            geometries: self.geometries,
            strings: self.strings,
            outgoing_offsets,
            outgoing_edges,
            incoming_offsets,
            incoming_edges,
            turn_restrictions: self.turn_restrictions,
        }
    }
}

pub fn build_csr(edges: &[Edge], node_count: usize) -> (Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>) {
    let flatten = |key: &dyn Fn(&Edge) -> u32| {
        let mut offsets = vec![0u32; node_count + 1];
        for edge in edges {
            offsets[key(edge) as usize + 1] += 1;
        }
        for index in 0..node_count {
            offsets[index + 1] += offsets[index];
        }
        let mut cursor = offsets.clone();
        let mut values = vec![0u32; edges.len()];
        for (index, edge) in edges.iter().enumerate() {
            let slot = &mut cursor[key(edge) as usize];
            values[*slot as usize] = index as u32;
            *slot += 1;
        }
        (offsets, values)
    };
    let (outgoing_offsets, outgoing_edges) = flatten(&|edge| edge.from);
    let (incoming_offsets, incoming_edges) = flatten(&|edge| edge.to);
    (
        outgoing_offsets,
        outgoing_edges,
        incoming_offsets,
        incoming_edges,
    )
}

/// Graph with indexes built once when loaded.
pub struct PreparedGraph {
    graph: Graph,
    main_component_size: usize,
    grid: HashMap<(i32, i32), Vec<u32>>,
    restrictions: HashMap<(u32, i64), Vec<(i64, bool)>>,
    /// Initial and final bearing of each edge in its traversal direction.
    bearings: Vec<(f32, f32)>,
}

#[derive(Clone, Copy, Debug)]
struct Snap {
    edge: u32,
    /// Position on the stored geometry: segment index plus fraction.
    position: f64,
    point: Coordinate,
    distance: f64,
}

struct VirtualEdge {
    from: u32,
    to: u32,
    source: u32,
    distance: f64,
    geometry: Vec<Coordinate>,
    bearings: (f32, f32),
}

/// Temporary nodes and edges for a query. The base graph is not modified.
struct Query {
    origin: u32,
    destination: u32,
    coordinates: [Coordinate; 2],
    edges: Vec<VirtualEdge>,
}

struct Path {
    edges: Vec<u32>,
    distance: f64,
}

#[derive(Debug, Clone, Copy)]
struct QueueEntry {
    priority: f64,
    cost: f64,
    edge: u32,
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for QueueEntry {}
impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .total_cmp(&self.priority)
            .then_with(|| other.edge.cmp(&self.edge))
    }
}
impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PreparedGraph {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        Graph::from_bytes(bytes).map(Self::new)
    }

    pub fn new(graph: Graph) -> Self {
        let in_main_component = largest_strong_component(&graph);
        let main_component_size = in_main_component.iter().filter(|value| **value).count();
        let bearings = graph
            .edges
            .iter()
            .map(|edge| {
                let geometry = &graph.geometries[edge.geometry as usize];
                let (start, end) = geometry_bearings(geometry);
                if edge.reversed {
                    (reverse_bearing(end), reverse_bearing(start))
                } else {
                    (start, end)
                }
            })
            .collect();
        let mut restrictions: HashMap<(u32, i64), Vec<(i64, bool)>> = HashMap::new();
        for restriction in &graph.turn_restrictions {
            restrictions
                .entry((restriction.via_node, restriction.from_way))
                .or_default()
                .push((restriction.to_way, restriction.only));
        }
        let mut grid: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        let mut indexed_geometries = vec![false; graph.geometries.len()];
        for (index, edge) in graph.edges.iter().enumerate() {
            if indexed_geometries[edge.geometry as usize]
                || !in_main_component[edge.from as usize]
                || !in_main_component[edge.to as usize]
            {
                continue;
            }
            indexed_geometries[edge.geometry as usize] = true;
            let geometry = &graph.geometries[edge.geometry as usize];
            let (mut min_x, mut min_y, mut max_x, mut max_y) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
            for point in geometry {
                let (x, y) = grid_cell(*point);
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            for x in min_x..=max_x {
                for y in min_y..=max_y {
                    grid.entry((x, y)).or_default().push(index as u32);
                }
            }
        }
        Self {
            graph,
            main_component_size,
            grid,
            restrictions,
            bearings,
        }
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn main_component_size(&self) -> usize {
        self.main_component_size
    }

    pub fn route(&self, request: &RouteRequest) -> Result<Vec<Route>, String> {
        let origin = self.snap(request.origin).ok_or(format!(
            "No bikeable road is within {} m of the origin",
            MAX_SNAP_METERS as u32
        ))?;
        let destination = self.snap(request.destination).ok_or(format!(
            "No bikeable road is within {} m of the destination",
            MAX_SNAP_METERS as u32
        ))?;
        if haversine_distance(origin.point, destination.point) < 1.0 {
            return Err("The origin and destination are too close together".into());
        }
        let query = self.build_query(origin, destination);
        let no_route = || "No bike route exists between these points".to_string();

        let preferred = self.search(&query, true, None).ok_or_else(no_route)?;
        let shortest = self.search(&query, false, None).ok_or_else(no_route)?;
        let primary_is_preferred =
            preferred.distance <= shortest.distance * MAX_PREFERRED_DETOUR_RATIO;
        let (primary, fallback) = if primary_is_preferred {
            (preferred, shortest)
        } else {
            (shortest, preferred)
        };

        let mut accepted = vec![primary];
        if request.alternatives > 0 {
            let mut candidates = vec![fallback];
            for penalty in ALTERNATIVE_PENALTIES {
                if accepted.len() > request.alternatives {
                    break;
                }
                for candidate in candidates.drain(..) {
                    self.accept_alternative(&query, &mut accepted, candidate);
                }
                if accepted.len() > request.alternatives {
                    break;
                }
                let mut penalties = vec![1.0f32; self.graph.geometries.len()];
                for path in &accepted {
                    for &edge in self.traveled_edges(&query, path) {
                        penalties[self.physical_geometry(&query, edge) as usize] = penalty;
                    }
                }
                if let Some(candidate) = self.search(&query, true, Some(&penalties)) {
                    candidates.push(candidate);
                }
            }
            for candidate in candidates {
                if accepted.len() > request.alternatives {
                    break;
                }
                self.accept_alternative(&query, &mut accepted, candidate);
            }
            accepted.truncate(request.alternatives + 1);
        }

        Ok(accepted
            .iter()
            .map(|path| self.route_from_path(&query, path))
            .collect())
    }

    fn accept_alternative(&self, query: &Query, accepted: &mut Vec<Path>, candidate: Path) {
        let primary_distance = accepted[0].distance;
        if candidate.distance > primary_distance * MAX_ALTERNATIVE_DISTANCE_RATIO {
            return;
        }
        let distinct = accepted.iter().all(|path| {
            candidate.edges != path.edges
                && self.overlap(query, path, &candidate) <= MAX_ALTERNATIVE_OVERLAP
        });
        if distinct {
            accepted.push(candidate);
        }
    }

    /// Fraction of `candidate` distance traveled on `reference` roads.
    fn overlap(&self, query: &Query, reference: &Path, candidate: &Path) -> f64 {
        let shared: HashSet<u32> = self
            .traveled_edges(query, reference)
            .map(|&edge| self.physical_geometry(query, edge))
            .collect();
        let shared_distance: f64 = self
            .traveled_edges(query, candidate)
            .filter(|&&edge| shared.contains(&self.physical_geometry(query, edge)))
            .map(|&edge| self.edge_distance(query, edge))
            .sum();
        shared_distance / candidate.distance.max(f64::EPSILON)
    }

    /// Edges with real length; zero-length snap links do not count as traveled road.
    fn traveled_edges<'a>(&'a self, query: &'a Query, path: &'a Path) -> impl Iterator<Item = &'a u32> {
        path.edges
            .iter()
            .filter(move |&&edge| self.edge_distance(query, edge) > 0.0)
    }

    fn snap(&self, point: Coordinate) -> Option<Snap> {
        let (cell_x, cell_y) = grid_cell(point);
        let cell_meters = GRID_CELL_DEGREES * METERS_PER_DEGREE;
        let radius_y = (MAX_SNAP_METERS / cell_meters).ceil() as i32;
        let radius_x = (MAX_SNAP_METERS / (cell_meters * point.lat.to_radians().cos().max(0.1)))
            .ceil() as i32;
        let mut best: Option<Snap> = None;
        let mut seen = HashSet::new();
        for x in (cell_x - radius_x)..=(cell_x + radius_x) {
            for y in (cell_y - radius_y)..=(cell_y + radius_y) {
                let Some(edges) = self.grid.get(&(x, y)) else {
                    continue;
                };
                for &edge_index in edges {
                    if !seen.insert(edge_index) {
                        continue;
                    }
                    let geometry =
                        &self.graph.geometries[self.graph.edges[edge_index as usize].geometry as usize];
                    for (segment, pair) in geometry.windows(2).enumerate() {
                        let (projected, t) = project_point(point, pair[0], pair[1]);
                        let distance = haversine_distance(point, projected);
                        if distance <= MAX_SNAP_METERS
                            && best.map(|snap| distance < snap.distance).unwrap_or(true)
                        {
                            best = Some(Snap {
                                edge: edge_index,
                                position: segment as f64 + t,
                                point: projected,
                                distance,
                            });
                        }
                    }
                }
            }
        }
        best
    }

    /// The given edge and, if present, its twin in the opposite direction.
    fn edge_and_twin(&self, edge_index: u32) -> Vec<u32> {
        let edge = &self.graph.edges[edge_index as usize];
        let mut edges = vec![edge_index];
        edges.extend(self.graph.outgoing(edge.to).iter().copied().filter(|&candidate| {
            let other = &self.graph.edges[candidate as usize];
            candidate != edge_index
                && other.geometry == edge.geometry
                && other.reversed != edge.reversed
                && other.to == edge.from
        }));
        edges
    }

    fn oriented_position(&self, edge_index: u32, stored_position: f64) -> (f64, usize) {
        let edge = &self.graph.edges[edge_index as usize];
        let last = self.graph.geometries[edge.geometry as usize].len() - 1;
        let position = if edge.reversed {
            last as f64 - stored_position
        } else {
            stored_position
        };
        (position, last)
    }

    fn build_query(&self, origin: Snap, destination: Snap) -> Query {
        let base_nodes = self.graph.nodes.len() as u32;
        let mut query = Query {
            origin: base_nodes,
            destination: base_nodes + 1,
            coordinates: [origin.point, destination.point],
            edges: Vec::new(),
        };
        let (origin_node, destination_node) = (query.origin, query.destination);
        let origin_edges = self.edge_and_twin(origin.edge);
        let destination_edges = self.edge_and_twin(destination.edge);
        for &edge_index in &origin_edges {
            let (position, last) = self.oriented_position(edge_index, origin.position);
            let to = self.graph.edges[edge_index as usize].to;
            self.push_virtual(&mut query, origin_node, to, edge_index, position, last as f64);
        }
        for &edge_index in &destination_edges {
            let (position, _) = self.oriented_position(edge_index, destination.position);
            let from = self.graph.edges[edge_index as usize].from;
            self.push_virtual(&mut query, from, destination_node, edge_index, 0.0, position);
        }
        for &edge_index in origin_edges.iter().filter(|edge| destination_edges.contains(edge)) {
            let (start, _) = self.oriented_position(edge_index, origin.position);
            let (end, _) = self.oriented_position(edge_index, destination.position);
            if start < end {
                self.push_virtual(&mut query, origin_node, destination_node, edge_index, start, end);
            }
        }
        query
    }

    fn push_virtual(&self, query: &mut Query, from: u32, to: u32, source: u32, start: f64, end: f64) {
        let edge = &self.graph.edges[source as usize];
        let full = self.graph.edge_geometry(edge);
        let geometry = slice_geometry(&full, start, end);
        let full_distance = geometry_distance(&full).max(f64::EPSILON);
        let distance = geometry_distance(&geometry) * edge.distance_meters as f64 / full_distance;
        let bearings = geometry_bearings(&geometry);
        query.edges.push(VirtualEdge {
            from,
            to,
            source,
            distance,
            geometry,
            bearings,
        });
    }

    fn base_edge_count(&self) -> u32 {
        self.graph.edges.len() as u32
    }

    fn source_edge<'a>(&'a self, query: &Query, edge: u32) -> &'a Edge {
        if edge < self.base_edge_count() {
            &self.graph.edges[edge as usize]
        } else {
            &self.graph.edges[query.edges[(edge - self.base_edge_count()) as usize].source as usize]
        }
    }

    fn physical_geometry(&self, query: &Query, edge: u32) -> u32 {
        self.source_edge(query, edge).geometry
    }

    fn edge_endpoints(&self, query: &Query, edge: u32) -> (u32, u32) {
        if edge < self.base_edge_count() {
            let edge = &self.graph.edges[edge as usize];
            (edge.from, edge.to)
        } else {
            let edge = &query.edges[(edge - self.base_edge_count()) as usize];
            (edge.from, edge.to)
        }
    }

    fn edge_distance(&self, query: &Query, edge: u32) -> f64 {
        if edge < self.base_edge_count() {
            self.graph.edges[edge as usize].distance_meters as f64
        } else {
            query.edges[(edge - self.base_edge_count()) as usize].distance
        }
    }

    fn edge_bearings(&self, query: &Query, edge: u32) -> (f32, f32) {
        if edge < self.base_edge_count() {
            self.bearings[edge as usize]
        } else {
            query.edges[(edge - self.base_edge_count()) as usize].bearings
        }
    }

    fn node_coordinate(&self, query: &Query, node: u32) -> Coordinate {
        match self.graph.nodes.get(node as usize) {
            Some(node) => node.coordinate,
            None => query.coordinates[(node - query.origin) as usize],
        }
    }

    fn outgoing_edges(&self, query: &Query, node: u32, output: &mut Vec<u32>) {
        output.clear();
        if (node as usize) < self.graph.nodes.len() {
            output.extend_from_slice(self.graph.outgoing(node));
        }
        output.extend(
            query
                .edges
                .iter()
                .enumerate()
                .filter(|(_, edge)| edge.from == node)
                .map(|(index, _)| self.base_edge_count() + index as u32),
        );
    }

    fn forbidden_turn(&self, query: &Query, incoming: u32, outgoing: u32, via: u32) -> bool {
        if via as usize >= self.graph.nodes.len() {
            return false;
        }
        let (Some(from_way), Some(to_way)) = (
            self.source_edge(query, incoming).osm_way_id,
            self.source_edge(query, outgoing).osm_way_id,
        ) else {
            return false;
        };
        let Some(restrictions) = self.restrictions.get(&(via, from_way)) else {
            return false;
        };
        let mut has_only = false;
        for &(restricted_way, only) in restrictions {
            if only {
                has_only = true;
                if restricted_way == to_way {
                    return false;
                }
            } else if restricted_way == to_way {
                // `no_u_turn` on the same road only prohibits turning back, not going straight.
                if from_way != to_way
                    || turn_angle(
                        self.edge_bearings(query, incoming).1,
                        self.edge_bearings(query, outgoing).0,
                    ) > 150.0
                {
                    return true;
                }
            }
        }
        has_only
    }

    /// A* over edge states: continuation cost depends on the incoming edge.
    fn search(&self, query: &Query, weighted: bool, penalties: Option<&[f32]>) -> Option<Path> {
        let total_edges = self.graph.edges.len() + query.edges.len();
        let mut costs = vec![f64::INFINITY; total_edges];
        let mut previous = vec![NO_EDGE; total_edges];
        let mut queue = BinaryHeap::new();
        let mut neighbors = Vec::new();
        let target = self.node_coordinate(query, query.destination);
        let edge_cost = |edge: u32| {
            let source = self.source_edge(query, edge);
            let factor = if weighted {
                (source.routing_cost as f64).max(1.0)
            } else {
                1.0
            };
            let penalty = penalties
                .map(|penalties| penalties[source.geometry as usize] as f64)
                .unwrap_or(1.0);
            self.edge_distance(query, edge) * factor * penalty
        };
        let heuristic = |node: u32| haversine_distance(self.node_coordinate(query, node), target);

        self.outgoing_edges(query, query.origin, &mut neighbors);
        for &edge in &neighbors {
            let cost = edge_cost(edge);
            if cost < costs[edge as usize] {
                costs[edge as usize] = cost;
                let (_, to) = self.edge_endpoints(query, edge);
                queue.push(QueueEntry {
                    priority: cost + heuristic(to),
                    cost,
                    edge,
                });
            }
        }

        while let Some(QueueEntry { cost, edge, .. }) = queue.pop() {
            if cost > costs[edge as usize] {
                continue;
            }
            let (_, node) = self.edge_endpoints(query, edge);
            if node == query.destination {
                let mut edges = vec![edge];
                let mut current = edge;
                while previous[current as usize] != NO_EDGE {
                    current = previous[current as usize];
                    edges.push(current);
                }
                edges.reverse();
                let distance = edges.iter().map(|&edge| self.edge_distance(query, edge)).sum();
                return Some(Path { edges, distance });
            }
            let incoming_bearing = self.edge_bearings(query, edge).1;
            self.outgoing_edges(query, node, &mut neighbors);
            for &next in &neighbors {
                if self.forbidden_turn(query, edge, next, node) {
                    continue;
                }
                let turn_cost = turn_penalty(turn_angle(
                    incoming_bearing,
                    self.edge_bearings(query, next).0,
                ));
                let next_cost = cost + edge_cost(next) + turn_cost;
                if next_cost < costs[next as usize] {
                    costs[next as usize] = next_cost;
                    previous[next as usize] = edge;
                    let (_, to) = self.edge_endpoints(query, next);
                    queue.push(QueueEntry {
                        priority: next_cost + heuristic(to),
                        cost: next_cost,
                        edge: next,
                    });
                }
            }
        }
        None
    }

    fn route_from_path(&self, query: &Query, path: &Path) -> Route {
        let mut segments: Vec<RouteSegment> = Vec::new();
        let mut keys: Vec<(Infrastructure, Option<u32>, Option<u32>)> = Vec::new();
        for &edge in &path.edges {
            let source = self.source_edge(query, edge);
            let geometry = if edge < self.base_edge_count() {
                self.graph.edge_geometry(source)
            } else {
                query.edges[(edge - self.base_edge_count()) as usize].geometry.clone()
            };
            let distance = self.edge_distance(query, edge);
            let key = (source.infrastructure, source.road_name, source.bicycle_access);
            if keys.last() == Some(&key) {
                let segment = segments.last_mut().unwrap();
                segment.geometry.extend(geometry.into_iter().skip(1));
                segment.distance_meters += distance;
                continue;
            }
            keys.push(key);
            segments.push(RouteSegment {
                geometry,
                distance_meters: distance,
                infrastructure: source.infrastructure,
                road_name: self.graph.string(source.road_name).map(str::to_string),
                bicycle_access: self.graph.string(source.bicycle_access).map(str::to_string),
            });
        }
        if segments.iter().any(|segment| segment.distance_meters > 0.0) {
            segments.retain(|segment| segment.distance_meters > 0.0);
        }
        Route {
            distance_meters: path.distance,
            cycleway_meters: segments
                .iter()
                .filter(|segment| segment.infrastructure == Infrastructure::Cycleway)
                .map(|segment| segment.distance_meters)
                .sum(),
            segments,
        }
    }
}

/// Largest strongly connected component (iterative Kosaraju).
fn largest_strong_component(graph: &Graph) -> Vec<bool> {
    let node_count = graph.nodes.len();
    let mut visited = vec![false; node_count];
    let mut order = Vec::with_capacity(node_count);
    for start in 0..node_count as u32 {
        if visited[start as usize] {
            continue;
        }
        visited[start as usize] = true;
        let mut stack = vec![(start, 0usize)];
        while let Some((node, cursor)) = stack.last_mut() {
            let outgoing = graph.outgoing(*node);
            if *cursor < outgoing.len() {
                let next = graph.edges[outgoing[*cursor] as usize].to;
                *cursor += 1;
                if !visited[next as usize] {
                    visited[next as usize] = true;
                    stack.push((next, 0));
                }
            } else {
                order.push(*node);
                stack.pop();
            }
        }
    }

    let mut component = vec![u32::MAX; node_count];
    let mut sizes = Vec::new();
    for &start in order.iter().rev() {
        if component[start as usize] != u32::MAX {
            continue;
        }
        let id = sizes.len() as u32;
        let mut size = 0usize;
        component[start as usize] = id;
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            size += 1;
            for &edge in graph.incoming(node) {
                let previous = graph.edges[edge as usize].from;
                if component[previous as usize] == u32::MAX {
                    component[previous as usize] = id;
                    stack.push(previous);
                }
            }
        }
        sizes.push(size);
    }
    let largest = sizes
        .iter()
        .enumerate()
        .max_by_key(|(_, size)| **size)
        .map(|(id, _)| id as u32);
    component
        .into_iter()
        .map(|id| Some(id) == largest)
        .collect()
}

fn grid_cell(point: Coordinate) -> (i32, i32) {
    (
        (point.lon / GRID_CELL_DEGREES).floor() as i32,
        (point.lat / GRID_CELL_DEGREES).floor() as i32,
    )
}

fn project_point(point: Coordinate, start: Coordinate, end: Coordinate) -> (Coordinate, f64) {
    let latitude_scale = point.lat.to_radians().cos().max(0.1);
    let (px, py) = (point.lon * latitude_scale, point.lat);
    let (sx, sy) = (start.lon * latitude_scale, start.lat);
    let (ex, ey) = (end.lon * latitude_scale, end.lat);
    let (dx, dy) = (ex - sx, ey - sy);
    let denominator = dx * dx + dy * dy;
    let t = if denominator == 0.0 {
        0.0
    } else {
        (((px - sx) * dx + (py - sy) * dy) / denominator).clamp(0.0, 1.0)
    };
    (interpolate(start, end, t), t)
}

fn interpolate(start: Coordinate, end: Coordinate, t: f64) -> Coordinate {
    Coordinate {
        lon: start.lon + (end.lon - start.lon) * t,
        lat: start.lat + (end.lat - start.lat) * t,
    }
}

fn point_at(geometry: &[Coordinate], position: f64) -> Coordinate {
    let last_segment = geometry.len() - 2;
    let segment = (position.floor().max(0.0) as usize).min(last_segment);
    interpolate(
        geometry[segment],
        geometry[segment + 1],
        (position - segment as f64).clamp(0.0, 1.0),
    )
}

/// Sub-polyline between two positions (segment index plus fraction), with `start <= end`.
fn slice_geometry(geometry: &[Coordinate], start: f64, end: f64) -> Vec<Coordinate> {
    let mut output = vec![point_at(geometry, start)];
    output.extend(
        geometry
            .iter()
            .enumerate()
            .filter(|(index, _)| (*index as f64) > start && (*index as f64) < end)
            .map(|(_, point)| *point),
    );
    output.push(point_at(geometry, end));
    output
}

fn geometry_distance(geometry: &[Coordinate]) -> f64 {
    geometry
        .windows(2)
        .map(|pair| haversine_distance(pair[0], pair[1]))
        .sum()
}

fn geometry_bearings(geometry: &[Coordinate]) -> (f32, f32) {
    let first = geometry[0];
    let last = geometry[geometry.len() - 1];
    let start = geometry
        .iter()
        .find(|point| **point != first)
        .map(|point| bearing(first, *point))
        .unwrap_or(0.0);
    let end = geometry
        .iter()
        .rev()
        .find(|point| **point != last)
        .map(|point| bearing(*point, last))
        .unwrap_or(start);
    (start, end)
}

fn reverse_bearing(bearing: f32) -> f32 {
    let reversed = bearing + std::f32::consts::PI;
    if reversed > std::f32::consts::PI {
        reversed - 2.0 * std::f32::consts::PI
    } else {
        reversed
    }
}

fn bearing(start: Coordinate, end: Coordinate) -> f32 {
    let latitude = ((start.lat + end.lat) / 2.0).to_radians().cos().max(0.1);
    (end.lat - start.lat).atan2((end.lon - start.lon) * latitude) as f32
}

/// Absolute turn angle in degrees, from 0 (straight) to 180 (U-turn).
fn turn_angle(incoming: f32, outgoing: f32) -> f64 {
    let mut delta = (outgoing - incoming).abs() as f64;
    if delta > std::f64::consts::PI {
        delta = 2.0 * std::f64::consts::PI - delta;
    }
    delta.to_degrees()
}

fn turn_penalty(degrees: f64) -> f64 {
    if degrees < 25.0 {
        0.0
    } else if degrees < 75.0 {
        4.0
    } else if degrees < 135.0 {
        10.0
    } else {
        18.0
    }
}

pub fn haversine_distance(a: Coordinate, b: Coordinate) -> f64 {
    let lat1 = a.lat.to_radians();
    let lat2 = b.lat.to_radians();
    let delta_lat = (b.lat - a.lat).to_radians();
    let delta_lon = (b.lon - a.lon).to_radians();
    let value =
        (delta_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
    6_371_000.0 * 2.0 * value.sqrt().min(1.0).asin()
}

#[cfg(test)]
mod tests;
