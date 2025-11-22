use std::collections::HashMap;

pub struct SafetyMap;

impl SafetyMap {
    pub fn new() -> Self {
        Self
    }

    pub fn calculate_edge_risk(&self, tags: &HashMap<&str, &str>) -> f32 {
        // 1. BASELINE RISK
        let highway_type = tags.get("highway").copied().unwrap_or("");
        
        let mut score: f32 = match highway_type {
            "pedestrian" | "footway" | "path" | "steps" => 0.1, 
            "living_street" | "residential" => 0.3, 
            "service" => 0.5,
            "tertiary" | "secondary" => 0.7,
            "primary" | "trunk" => 0.9, 
            _ => 0.5,
        };

        // 2. FEATURE WEIGHTS
        if let Some(lit) = tags.get("lit") {
            match *lit {
                "yes" | "24/7" | "automatic" | "good" => score -= 0.2,
                "no" => score += 0.3,
                _ => {}
            }
        }

        if let Some(sidewalk) = tags.get("sidewalk") {
            match *sidewalk {
                "both" | "yes" | "separate" | "left" | "right" => score -= 0.2,
                "no" | "none" => score += 0.2,
                _ => {}
            }
        }

        if let Some(surface) = tags.get("surface") {
            match *surface {
                "paved" | "asphalt" | "concrete" | "paving_stones" => score -= 0.05,
                "unpaved" | "dirt" | "earth" | "gravel" | "mud" => score += 0.1,
                _ => {}
            }
        }

        if let Some(foot) = tags.get("foot") {
            if *foot == "designated" {
                score -= 0.1;
            }
        }

        // 3. CLAMPING
        score.clamp(0.05, 1.0)
    }
}
