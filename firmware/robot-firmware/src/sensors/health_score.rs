//! Composite Plant & Soil Health Scoring Algorithm
//!
//! Computes a 0-1000 health score based on weighted sensor inputs:
//! - NDVI (30%): Vegetation vigor
//! - Leaf Temperature (15%): Water stress indicator
//! - Soil Moisture (20%): Water availability
//! - NPK Balance (15%): Nutrient availability
//! - pH (10%): Soil acidity
//! - EC (10%): Salinity / fertilizer concentration

use crate::sensors::{PlantData, SoilData};

/// Crop-specific optimal ranges (configurable per crop type)
#[derive(Debug, Clone)]
pub struct CropProfile {
    /// Crop name
    pub name: &'static str,
    /// Optimal NDVI range (× 1000)
    pub ndvi_min: i16,
    pub ndvi_max: i16,
    /// Optimal leaf temp relative to ambient (°C × 100)
    pub leaf_temp_delta_min: i16,
    pub leaf_temp_delta_max: i16,
    /// Optimal soil moisture (% × 10)
    pub moisture_min: u16,
    pub moisture_max: u16,
    /// Optimal nitrogen (mg/kg)
    pub nitrogen_min: u16,
    pub nitrogen_max: u16,
    /// Optimal phosphorus (mg/kg)
    pub phosphorus_min: u16,
    pub phosphorus_max: u16,
    /// Optimal potassium (mg/kg)
    pub potassium_min: u16,
    pub potassium_max: u16,
    /// Optimal pH (× 100)
    pub ph_min: u16,
    pub ph_max: u16,
    /// Optimal EC (µS/cm)
    pub ec_min: u16,
    pub ec_max: u16,
}

/// Default crop profile (general agriculture)
pub const DEFAULT_PROFILE: CropProfile = CropProfile {
    name: "General",
    ndvi_min: 400,   // 0.4
    ndvi_max: 900,   // 0.9
    leaf_temp_delta_min: -300, // -3.0°C (healthy transpiration)
    leaf_temp_delta_max: 200,  // +2.0°C
    moisture_min: 200, // 20%
    moisture_max: 600, // 60%
    nitrogen_min: 50,
    nitrogen_max: 300,
    phosphorus_min: 20,
    phosphorus_max: 150,
    potassium_min: 100,
    potassium_max: 400,
    ph_min: 550,  // 5.5
    ph_max: 750,  // 7.5
    ec_min: 200,  // 200 µS/cm
    ec_max: 4000, // 4000 µS/cm
};

/// Rice paddy profile
pub const RICE_PROFILE: CropProfile = CropProfile {
    name: "Rice",
    ndvi_min: 500,
    ndvi_max: 900,
    leaf_temp_delta_min: -200,
    leaf_temp_delta_max: 300,
    moisture_min: 500, // Rice needs saturated soil
    moisture_max: 900,
    nitrogen_min: 80,
    nitrogen_max: 250,
    phosphorus_min: 15,
    phosphorus_max: 100,
    potassium_min: 80,
    potassium_max: 300,
    ph_min: 550,
    ph_max: 700,
    ec_min: 100,
    ec_max: 3000,
};

/// Wheat profile
pub const WHEAT_PROFILE: CropProfile = CropProfile {
    name: "Wheat",
    ndvi_min: 400,
    ndvi_max: 850,
    leaf_temp_delta_min: -300,
    leaf_temp_delta_max: 400,
    moisture_min: 200,
    moisture_max: 500,
    nitrogen_min: 60,
    nitrogen_max: 200,
    phosphorus_min: 20,
    phosphorus_max: 120,
    potassium_min: 80,
    potassium_max: 250,
    ph_min: 600,
    ph_max: 750,
    ec_min: 200,
    ec_max: 3500,
};

/// Compute composite health score (0-1000)
pub fn compute_health_score(plant: &PlantData, soil: &SoilData) -> u16 {
    compute_health_score_with_profile(plant, soil, &DEFAULT_PROFILE)
}

/// Compute health score with a specific crop profile
pub fn compute_health_score_with_profile(
    plant: &PlantData,
    soil: &SoilData,
    profile: &CropProfile,
) -> u16 {
    // Each sub-score is 0-1000

    // NDVI score (30%)
    let ndvi_score = range_score(plant.ndvi as i32, profile.ndvi_min as i32, profile.ndvi_max as i32);

    // Leaf temperature score (15%) - based on delta from expected range
    let leaf_score = range_score(
        plant.leaf_temp as i32,
        profile.leaf_temp_delta_min as i32,
        profile.leaf_temp_delta_max as i32,
    );

    // Soil moisture score (20%)
    let moisture_score = range_score(
        soil.moisture as i32,
        profile.moisture_min as i32,
        profile.moisture_max as i32,
    );

    // NPK balance score (15%) - average of N, P, K sub-scores
    let n_score = range_score(
        soil.nitrogen as i32,
        profile.nitrogen_min as i32,
        profile.nitrogen_max as i32,
    );
    let p_score = range_score(
        soil.phosphorus as i32,
        profile.phosphorus_min as i32,
        profile.phosphorus_max as i32,
    );
    let k_score = range_score(
        soil.potassium as i32,
        profile.potassium_min as i32,
        profile.potassium_max as i32,
    );
    let npk_score = (n_score + p_score + k_score) / 3;

    // pH score (10%)
    let ph_score = range_score(soil.ph as i32, profile.ph_min as i32, profile.ph_max as i32);

    // EC score (10%)
    let ec_score = range_score(soil.ec as i32, profile.ec_min as i32, profile.ec_max as i32);

    // Weighted composite
    let composite = (ndvi_score as u32 * 30
        + leaf_score as u32 * 15
        + moisture_score as u32 * 20
        + npk_score as u32 * 15
        + ph_score as u32 * 10
        + ec_score as u32 * 10)
        / 100;

    composite.min(1000) as u16
}

/// Compute a 0-1000 score for a value within an optimal range.
/// Score is 1000 when value is within [min, max].
/// Degrades linearly outside the range.
fn range_score(value: i32, min: i32, max: i32) -> u16 {
    if value >= min && value <= max {
        return 1000;
    }

    let range = (max - min).max(1);
    let margin = range / 2; // Tolerance margin (50% of range)

    if value < min {
        let deficit = min - value;
        if deficit > margin {
            0
        } else {
            ((margin - deficit) as u32 * 1000 / margin as u32) as u16
        }
    } else {
        // value > max
        let excess = value - max;
        if excess > margin {
            0
        } else {
            ((margin - excess) as u32 * 1000 / margin as u32) as u16
        }
    }
}
