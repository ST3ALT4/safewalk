use std::collections::HashMap;
use petgraph::graph::{Graph, NodeIndex};
use osmpbf::{ElementReader, Element};
use geo::prelude::*;
use geo::Point;
use crate::safety::SafetyMap;

#[derive(Debug, Clone, Copy)]
pub struct GeoNode {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct WalkEdge {
    pub distance_meters: f64,
    pub safety_score: f32, 
}

pub struct NavigationGraph {
    pub graph: Graph<GeoNode, WalkEdge>,
    // Helper to lookup graph NodeIndex by OSM Node ID
    pub osm_id_map: HashMap<i64, NodeIndex>, 
}

impl NavigationGraph {
    pub fn from_pbf(path: &str, safety_map: &SafetyMap) -> anyhow::Result<Self> {
        println!("Parsing OSM PBF...");
        
        let mut graph = Graph::new();
        let mut osm_id_map = HashMap::new();
        // ID -> (lat, lon)
        // Note: For country-scale maps, this HashMap can get huge. 
        // For production systems on large areas, consider using a disk-backed store (like sled) here.
        let mut temp_nodes = HashMap::new(); 

        // PASS 1: Store all Nodes
        // We need to do this first so when we see a "Way" (Edge), we know where the points are.
        let reader = ElementReader::from_path(path)?;
        let mut node_count = 0;

        reader.for_each(|element| {
        match element {
            // Standard Node (rare in PBF)
            Element::Node(node) => {
                temp_nodes.insert(node.id(), (node.lat(), node.lon()));
                node_count += 1;
            }
            // DenseNode (Common - THIS WAS MISSING)
            Element::DenseNode(node) => {
                temp_nodes.insert(node.id(), (node.lat(), node.lon()));
                node_count += 1;
            }
            _ => {} // Ignore Ways and Relations during Pass 1
        }
        })?;

        println!("Real node count: {}", node_count);
        println!("Nodes loaded in map ({}). Building Edges...", temp_nodes.len());        
        
        // PASS 2: Build Ways
        let reader_pass2 = ElementReader::from_path(path)?;
        reader_pass2.for_each(|element| {
            if let Element::Way(way) = element {
                // 1. Extract Tags efficiently by iterating once
                let mut highway = "";
                let mut foot = "";
                let mut sidewalk = "";

                for (key, value) in way.tags() {
                    match key {
                        "highway" => highway = value,
                        "foot" => foot = value,
                        "sidewalk" => sidewalk = value,
                        _ => {}
                    }
                }

                // 2. Filter Logic
                // Standard walkable path types
                let is_walkable_type = matches!(highway, 
                    "footway" | "path" | "steps" | "pedestrian" | "living_street" | 
                    "residential" | "tertiary" | "service" | "unclassified"
                );

                // Logic for high-speed roads (allow ONLY if explicit infrastructure exists)
                let is_motor_road = matches!(highway, "motorway" | "trunk" | "primary" | "secondary");
                
                // Check for explicit walking permissions
                // "yes", "designated", "permissive" are standard OSM values for allowed access
                let foot_allowed = matches!(foot, "yes" | "designated" | "permissive");
                
                // Check for sidewalk presence
                // "both", "left", "right", "yes", "separate" indicate a sidewalk
                let has_sidewalk = matches!(sidewalk, "both" | "left" | "right" | "yes" | "separate");

                let is_walkable = is_walkable_type || (is_motor_road && (foot_allowed || has_sidewalk));

                if is_walkable {
                    let refs: Vec<i64> = way.refs().collect();
                    
                    // Connect segments
                    for window in refs.windows(2) {
                        let id_a = window[0];
                        let id_b = window[1];

                        // Only add edge if we successfully found both nodes in Pass 1
                        if let (Some(&(lat_a, lon_a)), Some(&(lat_b, lon_b))) = (temp_nodes.get(&id_a), temp_nodes.get(&id_b)) {
                            
                            // Create Graph Nodes if they don't exist yet
                            // This handles intersections where nodes are reused between Ways
                            let idx_a = *osm_id_map.entry(id_a).or_insert_with(|| {
                                graph.add_node(GeoNode { lat: lat_a, lon: lon_a })
                            });
                            let idx_b = *osm_id_map.entry(id_b).or_insert_with(|| {
                                graph.add_node(GeoNode { lat: lat_b, lon: lon_b })
                            });

                            // Calculate Edge Metadata
                            let p1 = Point::new(lon_a, lat_a);
                            let p2 = Point::new(lon_b, lat_b);
                            let dist = p1.haversine_distance(&p2);
                            
                            // Average safety score of the two points
                            let safety_a = safety_map.get_risk_score(lat_a, lon_a);
                            let safety_b = safety_map.get_risk_score(lat_b, lon_b);
                            let avg_safety = (safety_a + safety_b) / 2.0;

                            let edge_data = WalkEdge {
                                distance_meters: dist,
                                safety_score: avg_safety,
                            };

                            // Add Bi-directional edges (Pedestrians can walk both ways)
                            graph.add_edge(idx_a, idx_b, edge_data);
                            graph.add_edge(idx_b, idx_a, edge_data);
                        }
                    }
                }
            }
        })?;

        println!("Graph built: {} nodes, {} edges", graph.node_count(), graph.edge_count());
        Ok(Self { graph, osm_id_map })
    }

    // Helper: Find nearest node (Simple Linear Scan for MVP)
    // OPTIMIZATION TODO: Replace with R-Tree (rstar crate) for production performance
    pub fn find_nearest_node(&self, lat: f64, lon: f64) -> Option<NodeIndex> {
        let target = Point::new(lon, lat);
        
        self.graph.node_indices()
            .min_by(|&a, &b| {
                let na = self.graph[a];
                let nb = self.graph[b];
                let da = Point::new(na.lon, na.lat).haversine_distance(&target);
                let db = Point::new(nb.lon, nb.lat).haversine_distance(&target);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}
