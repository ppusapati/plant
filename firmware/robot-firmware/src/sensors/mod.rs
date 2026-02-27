//! Sensor Subsystem
//!
//! Manages all sensor reading, calibration, and health scoring.
//! Publishes SensorPacket and PlantHealthPacket to communication channels.

pub mod health_score;

use defmt::*;
use crate::protocol::{PlantHealthPacket, SensorPacket};

/// Environmental data from BME280 + BH1750 + VEML6075
#[derive(Debug, Clone, Default, defmt::Format)]
pub struct EnvironmentData {
    /// Air temperature (°C × 100)
    pub air_temp: i16,
    /// Relative humidity (% × 100)
    pub humidity: u16,
    /// Barometric pressure (Pa)
    pub pressure: u32,
    /// Light intensity (lux)
    pub light_lux: u16,
    /// UV index (× 100)
    pub uv_index: u16,
    /// Wind speed (m/s × 100)
    pub wind_speed: u16,
    /// Rainfall cumulative (mm × 10)
    pub rainfall: u16,
}

/// Plant health data from cameras and IR thermometer
#[derive(Debug, Clone, Default, defmt::Format)]
pub struct PlantData {
    /// NDVI value (× 1000, range -1000 to 1000)
    pub ndvi: i16,
    /// Leaf temperature (°C × 100)
    pub leaf_temp: i16,
    /// Plant height (mm)
    pub plant_height: u16,
    /// Canopy coverage (% × 10)
    pub canopy_coverage: u16,
    /// Disease alert flag
    pub disease_alert: bool,
}

/// Soil data from RS485 Modbus sensors
#[derive(Debug, Clone, Default, defmt::Format)]
pub struct SoilData {
    /// Moisture (% × 10)
    pub moisture: u16,
    /// Temperature (°C × 100)
    pub temperature: i16,
    /// Nitrogen (mg/kg)
    pub nitrogen: u16,
    /// Phosphorus (mg/kg)
    pub phosphorus: u16,
    /// Potassium (mg/kg)
    pub potassium: u16,
    /// pH (× 100)
    pub ph: u16,
    /// EC (µS/cm)
    pub ec: u16,
}

/// Aggregate all sensor data into a protocol SensorPacket
pub fn build_sensor_packet(
    env: &EnvironmentData,
    soil: &SoilData,
) -> SensorPacket {
    SensorPacket {
        soil_moisture: soil.moisture,
        soil_temp: soil.temperature,
        soil_n: soil.nitrogen,
        soil_p: soil.phosphorus,
        soil_k: soil.potassium,
        soil_ph: soil.ph,
        soil_ec: soil.ec,
        air_temp: env.air_temp,
        humidity: env.humidity,
        pressure: env.pressure,
        light_lux: env.light_lux,
        uv_index: env.uv_index,
        wind_speed: env.wind_speed,
        rainfall: env.rainfall,
    }
}

/// Build plant health packet with composite health score
pub fn build_plant_health_packet(
    plant: &PlantData,
    soil: &SoilData,
) -> PlantHealthPacket {
    let score = health_score::compute_health_score(plant, soil);

    PlantHealthPacket {
        ndvi: plant.ndvi,
        leaf_temp: plant.leaf_temp,
        plant_height: plant.plant_height,
        canopy_coverage: plant.canopy_coverage,
        health_score: score,
        disease_alert: plant.disease_alert,
    }
}

/// NDVI calculation from dual-camera pixel data
/// NDVI = (NIR - RED) / (NIR + RED)
pub fn calculate_ndvi(red: u16, nir: u16) -> i16 {
    if red == 0 && nir == 0 {
        return 0;
    }

    let red_f = red as f32;
    let nir_f = nir as f32;
    let ndvi = (nir_f - red_f) / (nir_f + red_f);

    // Scale to integer: × 1000
    (ndvi * 1000.0) as i16
}

/// Average NDVI over a grid of pixels
pub fn calculate_ndvi_grid(
    red_pixels: &[u16],
    nir_pixels: &[u16],
    width: usize,
    height: usize,
    grid_size: usize,
) -> i16 {
    if red_pixels.len() != nir_pixels.len() || red_pixels.is_empty() {
        return 0;
    }

    let grid_w = width / grid_size;
    let grid_h = height / grid_size;
    let mut total_ndvi: i32 = 0;
    let mut count: i32 = 0;

    for gy in 0..grid_h {
        for gx in 0..grid_w {
            let mut red_sum: u32 = 0;
            let mut nir_sum: u32 = 0;
            let mut pixels = 0u32;

            for py in 0..grid_size {
                for px in 0..grid_size {
                    let x = gx * grid_size + px;
                    let y = gy * grid_size + py;
                    if x < width && y < height {
                        let idx = y * width + x;
                        if idx < red_pixels.len() {
                            red_sum += red_pixels[idx] as u32;
                            nir_sum += nir_pixels[idx] as u32;
                            pixels += 1;
                        }
                    }
                }
            }

            if pixels > 0 {
                let avg_red = (red_sum / pixels) as u16;
                let avg_nir = (nir_sum / pixels) as u16;
                total_ndvi += calculate_ndvi(avg_red, avg_nir) as i32;
                count += 1;
            }
        }
    }

    if count > 0 {
        (total_ndvi / count) as i16
    } else {
        0
    }
}
