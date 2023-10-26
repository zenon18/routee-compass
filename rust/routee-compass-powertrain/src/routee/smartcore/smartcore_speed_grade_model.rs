use crate::routee::prediction_model::SpeedGradePredictionModel;
use routee_compass_core::{
    model::traversal::traversal_model_error::TraversalModelError,
    util::unit::{as_f64::AsF64, EnergyRate, EnergyRateUnit, Grade, GradeUnit, Speed, SpeedUnit},
};
use smartcore::{
    ensemble::random_forest_regressor::RandomForestRegressor, linalg::basic::matrix::DenseMatrix,
};

pub struct SmartcoreSpeedGradeModel {
    rf: RandomForestRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>>,
    speed_unit: SpeedUnit,
    grade_unit: GradeUnit,
    energy_rate_unit: EnergyRateUnit,
}

impl SpeedGradePredictionModel for SmartcoreSpeedGradeModel {
    fn predict(
        &self,
        speed: Speed,
        speed_unit: SpeedUnit,
        grade: Grade,
        grade_unit: GradeUnit,
    ) -> Result<(EnergyRate, EnergyRateUnit), TraversalModelError> {
        let speed_value = speed_unit.convert(speed, self.speed_unit).as_f64();
        let grade_value = grade_unit.convert(grade, self.grade_unit).as_f64();
        let x = DenseMatrix::from_2d_vec(&vec![vec![speed_value, grade_value]]);
        let y = self
            .rf
            .predict(&x)
            .map_err(|e| TraversalModelError::PredictionModel(e.to_string()))?;

        let energy_rate = EnergyRate::new(y[0]);
        Ok((energy_rate, self.energy_rate_unit.clone()))
    }
}

impl SmartcoreSpeedGradeModel {
    pub fn new(
        routee_model_path: String,
        speed_unit: SpeedUnit,
        grade_unit: GradeUnit,
        energy_rate_unit: EnergyRateUnit,
    ) -> Result<Self, TraversalModelError> {
        // Load random forest binary file
        let rf_binary = std::fs::read(routee_model_path.clone()).map_err(|e| {
            TraversalModelError::FileReadError(routee_model_path.clone(), e.to_string())
        })?;
        let rf: RandomForestRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>> =
            bincode::deserialize(&rf_binary).map_err(|e| {
                TraversalModelError::FileReadError(routee_model_path.clone(), e.to_string())
            })?;
        Ok(SmartcoreSpeedGradeModel {
            rf,
            speed_unit,
            grade_unit,
            energy_rate_unit,
        })
    }
}