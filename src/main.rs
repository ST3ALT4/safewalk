mod safety;
mod graph;

use axum::{routing::{get, post}, Router, Json, extract::State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use petgraph::algo::astar;
use crate::graph::NavigationGraph;
use crate::safety::SafetyMap;

// Shared State for concurrency
struct AppState {
    nav_graph: NavigationGraph,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize Safety Data
    let safety_map = SafetyMap::new();

    // 2. Load OSM Data (Ensure you have patiala.osm.pbf in root)
    // If file missing, please download using the command provided in instructions
    let pbf_path = "assets/patiala.osm.pbf"; 
    let nav_graph = NavigationGraph::from_pbf(pbf_path, &safety_map)
        .expect("Failed to load PBF file. Did you download the OSM data?");

    let shared_state = Arc::new(AppState { nav_graph });

    // 3. Setup Router
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/route", post(calculate_route))
        .with_state(shared_state);

    println!("Server running on http://0.0.0.0:3000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// --- API DTOs ---

#[derive(Deserialize)]
struct RouteRequest {
    origin: [f64; 2],      // [lat, lon]
    destination: [f64; 2], // [lat, lon]
    alpha: f64,            // Safety preference (0.0 = fast, 5.0 = safe)
}

#[derive(Serialize)]
struct RouteResponse {
    geometry: GeoJsonLineString,
    total_distance: f64,
    average_safety: f32,
}

#[derive(Serialize)]
struct GeoJsonLineString {
    r#type: String,
    coordinates: Vec<[f64; 2]>, // [lon, lat] standard for GeoJSON
}

// --- Handler ---

async fn calculate_route(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RouteRequest>,
) -> Json<RouteResponse> {
    let g = &state.nav_graph.graph;

    // 1. Snap to Grid
    let start_node = state.nav_graph.find_nearest_node(payload.origin[0], payload.origin[1]).unwrap();
    let end_node = state.nav_graph.find_nearest_node(payload.destination[0], payload.destination[1]).unwrap();

    // 2. Calculate Route (Weighted A*)
    let path_result = astar(
        g,
        start_node,
        |finish| finish == end_node,
        |e| {
            let edge = e.weight();
            // COST FUNCTION: Distance * (1 + alpha * SafetyScore)
            // If safety_score is high (1.0) and alpha is 5, this edge costs 6x its length.
            edge.distance_meters * (1.0 + payload.alpha * edge.safety_score as f64)
        },
        |n| {
            // Heuristic: Euclidean distance (Admissible because cost >= distance)
            let node = g[n];
            let dest = g[end_node];
            // Simple approximate distance calculation
            let d_lat = node.lat - dest.lat;
            let d_lon = node.lon - dest.lon;
            (d_lat * d_lat + d_lon * d_lon).sqrt() * 111_000.0 
        },
    );

    // 3. Format Response
    match path_result {
        Some((_weighted_cost, nodes)) => {
            let mut coordinates = Vec::new();
            let mut real_distance = 0.0;

            // Iterate nodes to build coords and sum distance
            for (i, &node_idx) in nodes.iter().enumerate() {
                let node_data = g[node_idx];
                coordinates.push([node_data.lon, node_data.lat]);

                // Calculate distance from previous node
                if i > 0 {
                    let prev_idx = nodes[i-1];
                    // Find the edge between prev and current
                    if let Some(edge) = g.find_edge(prev_idx, node_idx) {
                        real_distance += g[edge].distance_meters;
                    }
                }
            }

            Json(RouteResponse {
                geometry: GeoJsonLineString {
                    r#type: "LineString".to_string(),
                    coordinates,
                },
                total_distance: real_distance, // Now returns real meters
                average_safety: 0.0, // You can calculate this similarly if needed
            })
        }
        // ... none case
        None => Json(RouteResponse {
            geometry: GeoJsonLineString { r#type: "LineString".to_string(), coordinates: vec![] },
            total_distance: 0.0,
            average_safety: 0.0,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::NavigationGraph;
    use crate::safety::SafetyMap;
    use geo::prelude::*; // Ensure "geo" is in Cargo.toml

    #[test]
    fn test_patiala_route_thapar_to_omaxe() {
        // 1. Setup
        let pbf_path = "assets/patiala.osm.pbf"; // Make sure this file exists!
        if !std::path::Path::new(pbf_path).exists() {
            eprintln!("⚠️  Skipping test: assets/patiala.osm.pbf not found");
            return;
        }

        let safety_map = SafetyMap::new();
        let nav = NavigationGraph::from_pbf(pbf_path, &safety_map).unwrap();

        // 2. Coordinates
        // Thapar University (Approx Main Gate)
        let start_lat = 30.3515;
        let start_lon = 76.3700;
        
        // Omaxe Mall / Kali Devi Temple area
        let end_lat = 30.3410;
        let end_lon = 76.3940;

        // 3. Find Nodes
        let start_node = nav.find_nearest_node(start_lat, start_lon).expect("Start node not found");
        let end_node = nav.find_nearest_node(end_lat, end_lon).expect("End node not found");

        println!("Start Node Index: {:?}", start_node);
        println!("End Node Index: {:?}", end_node);

        // 4. Run Pathfinding
        let path = petgraph::algo::astar(
            &nav.graph,
            start_node,
            |finish| finish == end_node,
            |e| {
                // Use safety score multiplier
                e.weight().distance_meters * (1.0 + e.weight().safety_score as f64)
            },
            |n| {
                // Heuristic
                let node_data = nav.graph[n];
                let p1 = geo::Point::new(node_data.lon, node_data.lat);
                let p2 = geo::Point::new(end_lon, end_lat);
                p1.haversine_distance(&p2)
            }
        );

        // 5. Validate
        match path {
            Some((cost, nodes)) => {
                println!("✅ SUCCESS: Path found!");
                println!("Weighted Cost: {:.2}", cost);
                println!("Steps in path: {}", nodes.len());
                assert!(nodes.len() > 10, "Path should be reasonably long for this distance");
            },
            None => panic!("❌ FAILED: No path found between Thapar and Omaxe Mall"),
        }
    }
}
