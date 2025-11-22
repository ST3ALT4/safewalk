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
}

impl NavigationGraph {
    pub fn from_pbf(path: &str, safety_map: &SafetyMap) -> anyhow::Result<Self> {
        println!("Parsing OSM PBF: {}", path);
        
        let mut graph = Graph::new();
        let mut osm_id_map = HashMap::new();
        let mut temp_nodes = HashMap::new(); 

        // PASS 1: Nodes
        let reader = ElementReader::from_path(path)?;
        let mut node_count = 0;

        reader.for_each(|element| {
            match element {
                Element::Node(node) => {
                    temp_nodes.insert(node.id(), (node.lat(), node.lon()));
                    node_count += 1;
                }
                Element::DenseNode(node) => {
                    temp_nodes.insert(node.id(), (node.lat(), node.lon()));
                    node_count += 1;
                }
                _ => {} 
            }
        })?;

        println!("Loaded {} nodes. Building Edges...", node_count);        
        
        // PASS 2: Ways
        let reader_pass2 = ElementReader::from_path(path)?;
        reader_pass2.for_each(|element| {
            if let Element::Way(way) = element {
                
                let mut tags = HashMap::new();
                for (key, value) in way.tags() {
                    tags.insert(key, value);
                }

                let highway = tags.get("highway").copied().unwrap_or("");
                let foot = tags.get("foot").copied().unwrap_or("");
                let sidewalk = tags.get("sidewalk").copied().unwrap_or("");

                let is_walkable_type = matches!(highway, 
                    "footway" | "path" | "steps" | "pedestrian" | "living_street" | 
                    "residential" | "tertiary" | "service" | "unclassified"
                );

                let is_motor_road = matches!(highway, "motorway" | "trunk" | "primary" | "secondary");
                let foot_allowed = matches!(foot, "yes" | "designated" | "permissive");
                let has_sidewalk = matches!(sidewalk, "both" | "left" | "right" | "yes" | "separate");

                if is_walkable_type || (is_motor_road && (foot_allowed || has_sidewalk)) {
                    let risk_score = safety_map.calculate_edge_risk(&tags);

                    let refs: Vec<i64> = way.refs().collect();
                    
                    for window in refs.windows(2) {
                        let id_a = window[0];
                        let id_b = window[1];

                        if let (Some(&(lat_a, lon_a)), Some(&(lat_b, lon_b))) = (temp_nodes.get(&id_a), temp_nodes.get(&id_b)) {
                            
                            let idx_a = *osm_id_map.entry(id_a).or_insert_with(|| {
                                graph.add_node(GeoNode { lat: lat_a, lon: lon_a })
                            });
                            let idx_b = *osm_id_map.entry(id_b).or_insert_with(|| {
                                graph.add_node(GeoNode { lat: lat_b, lon: lon_b })
                            });

                            let p1 = Point::new(lon_a, lat_a);
                            let p2 = Point::new(lon_b, lat_b);
                            let dist = p1.haversine_distance(&p2);
                            
                            let edge_data = WalkEdge {
                                distance_meters: dist,
                                safety_score: risk_score,
                            };

                            graph.add_edge(idx_a, idx_b, edge_data);
                            graph.add_edge(idx_b, idx_a, edge_data);
                        }
                    }
                }
            }
        })?;

        println!("Graph built: {} nodes, {} edges", graph.node_count(), graph.edge_count());
        Ok(Self { graph }) 
    }

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
