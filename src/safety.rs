use h3o::{LatLng, Resolution, CellIndex};
use std::collections::HashMap;

// 0.0 = Safe (Well lit, populated), 1.0 = Dangerous (Dark, alleyway)
pub struct SafetyMap {
    cells: HashMap<CellIndex, f32>,
}

impl SafetyMap {
    pub fn new() -> Self {
        // MOCK DATA: Let's pretend some areas in Patiala are "risky" for the demo.
        // In production, you would load crime/lighting data here.
        let mut cells = HashMap::new();
        
        // Example: Create a "danger gradient" around a specific lat/lon
        let center_lat = 30.3398; 
        let center_lon = 76.3869;
        
        let center = LatLng::new(center_lat, center_lon).unwrap()
            .to_cell(Resolution::Nine); // Approx 174m resolution
            
        // Mark the center as risky (0.9) and neighbors as moderate (0.5)
        cells.insert(center, 0.9);
        for neighbor in center.grid_disk::<Vec<_>>(2) { // 2 rings of hexes
             cells.entry(neighbor).or_insert(0.4);
        }

        Self { cells }
    }

    pub fn get_risk_score(&self, lat: f64, lon: f64) -> f32 {
        // Resolution 10 is approx 65m edge length (good for street blocks)
        let cell = LatLng::new(lat, lon)
            .expect("Invalid coordinates")
            .to_cell(Resolution::Nine);
            
        // If we have data for this cell, return it. Default to 0.1 (base risk)
        *self.cells.get(&cell).unwrap_or(&0.1)
    }
}
