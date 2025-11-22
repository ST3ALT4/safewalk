# SafeWalk: Safe Geo-Navigation System 🚶‍♂️🛡️

**SafeWalk** is a high-performance navigation engine built with **Rust** that prioritizes pedestrian safety. Unlike standard maps that just find the shortest route, SafeWalk calculates paths based on a customizable "Safety Score," allowing users to avoid high-risk areas (e.g., unlit streets, high-crime zones) even if the path is slightly longer.

## 🌟 Features

* **Safety-First Routing:** Uses a weighted A* algorithm where `Cost = Distance * (1 + SafetyFactor)`.
* **High-Performance Backend:** Written in Rust using `axum` and `petgraph` for sub-millisecond pathfinding.
* **Real-World Data:** Parses OpenStreetMap PBF files directly (supports efficient `DenseNode` compression).
* **H3 Geospatial Indexing:** Uses Uber's H3 grid system to map risk scores to specific geographic zones (Resolution 9).

## 🛠️ Tech Stack

* **Backend:** Rust (Axum, Tokio, Petgraph, Osmpbf, Geo, H3o, Tower-HTTP)
* **Frontend:** HTML5, JavaScript, Leaflet.js (Hosted locally via Python)
* **Data Source:** OpenStreetMap Protocol Buffers (`.osm.pbf`)

---

## 📋 Prerequisites

1.  **Rust & Cargo:** [Install Rust](https://www.rust-lang.org/tools/install)
2.  **Python 3:** (Required only to serve the frontend UI)
3.  **Map Data:** A `.osm.pbf` file for your city.

## ⚙️ Installation & Setup

### 1. Clone the Repository
```bash
git clone <your-repo-url>
cd safe_walk
````

### 2\. Download Map Data

You need the raw OpenStreetMap data for the city you want to navigate.

1.  Go to [BBBike Extract Service](https://extract.bbbike.org/).
2.  Search for your city (e.g., "Patiala").
3.  Select Format: **Protocolbuffer (PBF)**.
4.  Download the file and rename it to `<city>.osm.pbf`.
5.  Place it in the `assets/` folder:
    ```text
    /assets/<city>.osm.pbf
    ```

### 3\. Build the Backend

```bash
cargo build --release
```

-----

## 🚀 How to Run

This system uses a **Detached UI architecture**. You need two terminals running at the same time.

### Terminal 1: Start the Backend (Rust API)

This loads the map graph and listens for route requests.

```bash
cargo run
```

*Wait until you see:* `🚀 API Server running on http://0.0.0.0:3000`

### Terminal 2: Start the Frontend (Map UI)

This serves the interactive map.

```bash
cd test
python3 -m http.server 8000
```

*Wait until you see:* `Serving HTTP on 0.0.0.0 port 8000 ...`

-----

## 🎮 How to Use

1.  Open your browser to **[http://localhost:8000](https://www.google.com/search?q=http://localhost:8000)**.
2.  **Set Start Point:** Click anywhere on the map (Green Marker).
3.  **Set Destination:** Click anywhere else on the map (Red Marker).
4.  **Adjust Safety:** Use the slider to set your preference:
      * `0.0` = Fastest Route (ignores safety risks).
      * `5.0+` = Safest Route (avoids risky zones aggressively).
5.  Click **"Find Safe Path"** to visualize the route.

-----

## 🔌 API Reference

The backend exposes a single REST endpoint used by the frontend.

**POST** `/route`

**Request Body:**

```json
{
  "origin": [30.3515, 76.3700],       // [Lat, Lon]
  "destination": [30.3410, 76.3940],  // [Lat, Lon]
  "alpha": 2.5                        // Safety Weight
}
```

**Response:**

```json
{
  "geometry": {
    "type": "LineString",
    "coordinates": [[76.3700, 30.3515], ...] // GeoJSON [Lon, Lat]
  },
  "total_distance": 3420.5, // In Meters
  "average_safety": 0.45    // 0.0 (Safe) -> 1.0 (Risky)
}
```

## 📂 Project Structure

```text
.
├── assets/
│   ├── index.html        # The Frontend Map Client
│   └── patiala.osm.pbf   # The Map Data (Not included in git)
├── src/
│   ├── main.rs           # API Server, CORS setup, & Route Handler
│   ├── graph.rs          # PBF Parser & Graph Builder (Nodes/Edges)
│   └── safety.rs         # Safety Map Logic (H3 Grid)
├── Cargo.toml            # Rust Dependencies
└── README.md
```

## 🔮 Future Roadmap

  * [ ] **Real Data Integration:** Connect `safety.rs` to real crime datasets or street lighting APIs.
  * [ ] **Turn-by-Turn Navigation:** Return text instructions (e.g., "Turn left on Bhadson Rd").
  * [ ] **Spatial Optimization:** Implement an R-Tree for faster `find_nearest_node` lookups on huge maps.


```
