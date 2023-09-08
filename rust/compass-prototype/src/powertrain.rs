use smartcore::{
    ensemble::random_forest_regressor::RandomForestRegressor, linalg::basic::matrix::DenseMatrix,
};

use anyhow::Result;
use pyo3::prelude::*;

use crate::{algorithm::compute_link_speed_kph, graph::Link, map::SearchInput};

// scale the energy or cost by this factor to make it an integer
pub const ROUTEE_SCALE_FACTOR: f64 = 1_000_000_000.0;

pub const CENTIMETERS_TO_MILES: f64 = 6.213712e-6;

#[pyclass]
#[derive(Clone, Copy, Debug)]
pub struct VehicleParameters {
    #[pyo3(get)]
    pub weight_lbs: u32,
    #[pyo3(get)]
    pub height_inches: u16,
    #[pyo3(get)]
    pub width_inches: u16,
    #[pyo3(get)]
    pub length_inches: u16,
    #[pyo3(get)]
    pub max_speed_kph: f64,
}

#[pymethods]
impl VehicleParameters {
    #[new]
    pub fn new(
        weight_lbs: u32,
        height_inches: u16,
        width_inches: u16,
        length_inches: u16,
        max_speed_kph: f64,
    ) -> Self {
        VehicleParameters {
            weight_lbs,
            height_inches,
            width_inches,
            length_inches,
            max_speed_kph,
        }
    }
}

pub fn compute_energy_over_path(path: &Vec<Link>, search_input: &SearchInput) -> Result<f64> {
    let model_file_path = search_input
        .routee_model_path
        .clone()
        .ok_or(anyhow::anyhow!(
            "routee_model_path must be set in SearchInput"
        ))?;
    let rf_binary = std::fs::read(model_file_path)?;

    let rf: RandomForestRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>> =
        bincode::deserialize(&rf_binary)?;

    let features = path
        .iter()
        .map(|link| {
            let vehicle_params = search_input.vehicle_parameters;
            let speed_kph = compute_link_speed_kph(link, search_input);
            let speed_mph = speed_kph * 0.621371;
            let grade_percent = link.grade as f64 / 10.0;

            match vehicle_params {
                Some(params) => vec![
                    speed_mph,
                    grade_percent,
                    params.weight_lbs as f64 * 0.453592,
                ],
                None => vec![speed_mph, grade_percent],
            }
        })
        .collect::<Vec<Vec<f64>>>();
    let x = DenseMatrix::from_2d_vec(&features);
    let energy_per_mile = rf.predict(&x).unwrap();
    let energy: f64 = energy_per_mile
        .iter()
        .zip(path.iter())
        .map(|(raw_energy_per_mile, link)| {
            let energy_per_mile = if *raw_energy_per_mile < 0.0 {
                0.0
            } else {
                *raw_energy_per_mile
            };
            let distance_miles = link.distance_centimeters as f64 * CENTIMETERS_TO_MILES;
            let mut energy = energy_per_mile * distance_miles;
            if link.stop_sign {
                energy = energy + search_input.stop_cost_gallons_diesel;
            }
            if link.traffic_light {
                // assume 50% of the time we stop at a traffic light
                let stop_cost = 0.5 * search_input.stop_cost_gallons_diesel;
                energy = energy + stop_cost;
            }
            energy
        })
        .sum();
    Ok(energy)
}

pub fn build_routee_cost_function_with_tods(
    search_input: SearchInput,
) -> Result<impl Fn(&Link) -> isize> {
    let model_file_path = search_input
        .routee_model_path
        .clone()
        .ok_or(anyhow::anyhow!(
            "routee_model_path must be set in SearchInput"
        ))?;
    let rf_binary = std::fs::read(model_file_path)?;

    let rf: RandomForestRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>> =
        bincode::deserialize(&rf_binary)?;

    Ok(move |link: &Link| {
        let distance_miles = link.distance_centimeters as f64 * CENTIMETERS_TO_MILES;
        let vehicle_params = search_input.vehicle_parameters;
        let speed_kph = compute_link_speed_kph(link, &search_input);
        let speed_mph = speed_kph * 0.621371;
        let grade_percent = link.grade as f64 / 10.0;

        let features = match vehicle_params {
            Some(params) => vec![vec![
                speed_mph,
                grade_percent,
                params.weight_lbs as f64 * 0.453592,
            ]],
            None => vec![vec![speed_mph, grade_percent]],
        };

        let x = DenseMatrix::from_2d_vec(&features);
        let raw_energy_per_mile = rf.predict(&x).unwrap()[0];

        let energy_per_mile = if raw_energy_per_mile < 0.0 {
            0.0
        } else {
            raw_energy_per_mile
        };

        let mut energy = energy_per_mile * distance_miles;
        if link.stop_sign {
            energy = energy + search_input.stop_cost_gallons_diesel;
        }
        if link.traffic_light {
            // assume 50% of the time we stop at a traffic light
            let stop_cost = 0.5 * search_input.stop_cost_gallons_diesel;
            energy = energy + stop_cost;
        }
        let scaled_energy = energy * ROUTEE_SCALE_FACTOR;
        scaled_energy.round() as isize
    })
}

pub fn build_routee_cost_function_with_cost(
    search_input: SearchInput,
    dollar_per_gallon: f64,
    dollar_per_hour: f64,
) -> Result<impl Fn(&Link) -> isize> {
    let model_file_path = search_input
        .routee_model_path
        .clone()
        .ok_or(anyhow::anyhow!(
            "routee_model_path must be set in SearchInput"
        ))?;
    let rf_binary = std::fs::read(model_file_path)?;

    let rf: RandomForestRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>> =
        bincode::deserialize(&rf_binary)?;

    Ok(move |link: &Link| {
        let distance_miles: f64 = link.distance_centimeters as f64 * CENTIMETERS_TO_MILES;
        let vehicle_params: Option<VehicleParameters> = search_input.vehicle_parameters;
        let speed_kph = compute_link_speed_kph(link, &search_input);
        let speed_mph = speed_kph * 0.621371;
        let grade_percent = link.grade as f64 / 10.0;
        let time_hours = distance_miles / speed_mph;

        let features: Vec<Vec<f64>> = match vehicle_params {
            Some(params) => vec![vec![
                speed_mph,
                grade_percent,
                params.weight_lbs as f64 * 0.453592,
            ]],
            None => vec![vec![speed_mph, grade_percent]],
        };

        let x: DenseMatrix<f64> = DenseMatrix::from_2d_vec(&features);
        let raw_energy_per_mile: f64 = rf.predict(&x).unwrap()[0];

        let energy_per_mile = if raw_energy_per_mile < 0.0 {
            0.0
        } else {
            raw_energy_per_mile
        };

        let mut energy = energy_per_mile * distance_miles;
        if link.stop_sign {
            energy = energy + search_input.stop_cost_gallons_diesel;
        }
        if link.traffic_light {
            // assume 50% of the time we stop at a traffic light
            let stop_cost = 0.5 * search_input.stop_cost_gallons_diesel;
            energy = energy + stop_cost;
        }
        let energy_dollars = energy * dollar_per_gallon;
        let time_dollars = time_hours * dollar_per_hour;
        let mixed_cost: f64 = energy_dollars + time_dollars;
        let scaled_cost = mixed_cost * ROUTEE_SCALE_FACTOR;
        scaled_cost.round() as isize
    })
}