use h3o::{LatLng, Resolution, CellIndex};
use std::collections::HashMap;

// 0.0 = Safe, 1.0 = Dangerous
pub struct SafetyMap {
    cells: HashMap<CellIndex, f32>,
}

impl SafetyMap {
    pub fn new() -> Self {
        let mut cells = HashMap::new();
        
        // MOCK DATA: Center of Patiala (Leela Bhawan area approx)
        let center_lat = 30.3398; 
        let center_lon = 76.3869;
        
        // Using Resolution NINE for both creation and query
        let center = LatLng::new(center_lat, center_lon).unwrap()
            .to_cell(Resolution::Nine); 
            
        // Mark the center as risky (0.9)
        cells.insert(center, 0.9);
        
        // Mark neighbors as moderate (0.4)
        for neighbor in center.grid_disk::<Vec<_>>(2) { 
             cells.entry(neighbor).or_insert(0.4);
        }

        Self { cells }
    }

    pub fn get_risk_score(&self, lat: f64, lon: f64) -> f32 {
        // FIX: Must match the resolution used in new() -> Resolution::Nine
        let cell = LatLng::new(lat, lon)
            .expect("Invalid coordinates")
            .to_cell(Resolution::Nine); 
            
        *self.cells.get(&cell).unwrap_or(&0.1)
    }
}
