mod safety;
mod graph;

use axum::{routing::{get, post}, Router, Json, extract::State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use petgraph::algo::astar;
use tower_http::cors::CorsLayer;
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

    // 2. Load OSM Data
    // Ensure "assets/patiala.osm.pbf" exists!
    let pbf_path = "assets/patiala.osm.pbf"; 
    let nav_graph = NavigationGraph::from_pbf(pbf_path, &safety_map)
        .expect("Failed to load PBF file. Check if assets/patiala.osm.pbf exists.");

    let shared_state = Arc::new(AppState { nav_graph });

    // 3. Setup CORS (Allows your local HTML file to talk to this API)
    let cors = CorsLayer::new()
        .allow_methods(tower_http::cors::Any)
        .allow_origin(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    // 4. Setup Router
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/route", post(calculate_route))
        .layer(cors)
        .with_state(shared_state);

    println!("🚀 API Server running on http://0.0.0.0:3000");
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

    // 1. Snap input coordinates to nearest Graph Nodes
    let start_node = state.nav_graph.find_nearest_node(payload.origin[0], payload.origin[1]);
    let end_node = state.nav_graph.find_nearest_node(payload.destination[0], payload.destination[1]);

    if start_node.is_none() || end_node.is_none() {
        return Json(RouteResponse {
            geometry: GeoJsonLineString { r#type: "LineString".to_string(), coordinates: vec![] },
            total_distance: 0.0,
            average_safety: 0.0,
        });
    }

    let start_node = start_node.unwrap();
    let end_node = end_node.unwrap();

    // 2. Calculate Route (Weighted A*)
    let path_result = astar(
        g,
        start_node,
        |finish| finish == end_node,
        |e| {
            let edge = e.weight();
            // COST FUNCTION: Distance * (1 + alpha * SafetyScore)
            // If alpha is high, dangerous edges become very "expensive"
            edge.distance_meters * (1.0 + payload.alpha * edge.safety_score as f64)
        },
        |n| {
            // Heuristic: Euclidean distance
            let node = g[n];
            let dest = g[end_node];
            let d_lat = node.lat - dest.lat;
            let d_lon = node.lon - dest.lon;
            (d_lat * d_lat + d_lon * d_lon).sqrt() * 111_000.0 
        },
    );

    // 3. Format Response
    match path_result {
        Some((_cost, nodes)) => {
            let mut coordinates = Vec::new();
            let mut real_distance = 0.0;
            let mut total_safety_score = 0.0;
            let mut edge_count = 0;

            // Reconstruct path geometry and stats
            for (i, &node_idx) in nodes.iter().enumerate() {
                let node_data = g[node_idx];
                coordinates.push([node_data.lon, node_data.lat]); // GeoJSON is [Lon, Lat]

                // Calculate distance and safety from edges
                if i > 0 {
                    let prev_idx = nodes[i-1];
                    if let Some(edge) = g.find_edge(prev_idx, node_idx) {
                        let weight = g[edge];
                        real_distance += weight.distance_meters;
                        total_safety_score += weight.safety_score;
                        edge_count += 1;
                    }
                }
            }
            
            let avg_safety = if edge_count > 0 { total_safety_score / edge_count as f32 } else { 0.0 };

            Json(RouteResponse {
                geometry: GeoJsonLineString {
                    r#type: "LineString".to_string(),
                    coordinates,
                },
                total_distance: real_distance,
                average_safety: avg_safety,
            })
        }
        None => Json(RouteResponse {
            geometry: GeoJsonLineString { r#type: "LineString".to_string(), coordinates: vec![] },
            total_distance: 0.0,
            average_safety: 0.0,
        })
    }
}
