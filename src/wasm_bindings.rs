use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use serde::Serialize;

use crate::*;

#[cfg(feature = "debug")]
#[wasm_bindgen(start)]
pub fn init_wasm() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct JsSolverConfig {
    strategy: String,
    default_domain_lo: Option<f64>,
    default_domain_hi: Option<f64>,
    penalty_weight: Option<f64>,
    montecarlo_n: Option<i32>,
    feasibility_tol: Option<f64>,
    popsize: Option<i32>,
    max_iter: Option<i32>,
}

#[wasm_bindgen]
impl JsSolverConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(strategy: String) -> Self {
        Self {
            strategy,
            default_domain_lo: None,
            default_domain_hi: None,
            penalty_weight: None,
            montecarlo_n: None,
            feasibility_tol: None,
            popsize: None,
            max_iter: None,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn strategy(&self) -> String { self.strategy.clone() }
    #[wasm_bindgen(setter)]
    pub fn set_strategy(&mut self, val: String) { self.strategy = val; }

    #[wasm_bindgen(getter)]
    pub fn default_domain_lo(&self) -> Option<f64> { self.default_domain_lo }
    #[wasm_bindgen(setter)]
    pub fn set_default_domain_lo(&mut self, val: Option<f64>) { self.default_domain_lo = val; }

    #[wasm_bindgen(getter)]
    pub fn default_domain_hi(&self) -> Option<f64> { self.default_domain_hi }
    #[wasm_bindgen(setter)]
    pub fn set_default_domain_hi(&mut self, val: Option<f64>) { self.default_domain_hi = val; }

    #[wasm_bindgen(getter)]
    pub fn penalty_weight(&self) -> Option<f64> { self.penalty_weight }
    #[wasm_bindgen(setter)]
    pub fn set_penalty_weight(&mut self, val: Option<f64>) { self.penalty_weight = val; }

    #[wasm_bindgen(getter)]
    pub fn montecarlo_n(&self) -> Option<i32> { self.montecarlo_n }
    #[wasm_bindgen(setter)]
    pub fn set_montecarlo_n(&mut self, val: Option<i32>) { self.montecarlo_n = val; }

    #[wasm_bindgen(getter)]
    pub fn feasibility_tol(&self) -> Option<f64> { self.feasibility_tol }
    #[wasm_bindgen(setter)]
    pub fn set_feasibility_tol(&mut self, val: Option<f64>) { self.feasibility_tol = val; }

    #[wasm_bindgen(getter)]
    pub fn popsize(&self) -> Option<i32> { self.popsize }
    #[wasm_bindgen(setter)]
    pub fn set_popsize(&mut self, val: Option<i32>) { self.popsize = val; }

    #[wasm_bindgen(getter)]
    pub fn max_iter(&self) -> Option<i32> { self.max_iter }
    #[wasm_bindgen(setter)]
    pub fn set_max_iter(&mut self, val: Option<i32>) { self.max_iter = val; }
}

#[wasm_bindgen]
pub struct JsSolverResult {
    feasible: bool,
    plan: JsValue,
    penalty: f64,
    strategy: String,
    probability: f64,
    effectiveness: f64,
    montecarlo_samples: i32,
    constraint_violations: Vec<String>,
    libertad: Vec<JsValue>,
    elapsed_ms: i32,
}

#[wasm_bindgen]
impl JsSolverResult {
    #[wasm_bindgen(getter)]
    pub fn feasible(&self) -> bool { self.feasible }

    #[wasm_bindgen(getter)]
    pub fn plan(&self) -> JsValue { self.plan.clone() }

    #[wasm_bindgen(getter)]
    pub fn penalty(&self) -> f64 { self.penalty }

    #[wasm_bindgen(getter)]
    pub fn strategy(&self) -> String { self.strategy.clone() }

    #[wasm_bindgen(getter)]
    pub fn probability(&self) -> f64 { self.probability }

    #[wasm_bindgen(getter)]
    pub fn effectiveness(&self) -> f64 { self.effectiveness }

    #[wasm_bindgen(getter)]
    pub fn montecarlo_samples(&self) -> i32 { self.montecarlo_samples }

    #[wasm_bindgen(getter)]
    pub fn constraint_violations(&self) -> Vec<String> { self.constraint_violations.clone() }

    #[wasm_bindgen(getter)]
    pub fn libertad(&self) -> Vec<JsValue> { self.libertad.clone() }

    #[wasm_bindgen(getter)]
    pub fn elapsed_ms(&self) -> i32 { self.elapsed_ms }
}

#[wasm_bindgen]
pub struct JsValidationResult {
    valid: bool,
    errors: Vec<String>,
}

#[wasm_bindgen]
impl JsValidationResult {
    #[wasm_bindgen(getter)]
    pub fn valid(&self) -> bool { self.valid }

    #[wasm_bindgen(getter)]
    pub fn errors(&self) -> Vec<String> { self.errors.clone() }
}

fn to_solver_config(cfg: &JsSolverConfig) -> SolverConfig {
    SolverConfig {
        strategy: Strategy::from_str(&cfg.strategy),
        default_domain: (
            cfg.default_domain_lo.unwrap_or(0.0),
            cfg.default_domain_hi.unwrap_or(100.0),
        ),
        penalty_weight: cfg.penalty_weight.unwrap_or(1e6),
        montecarlo_n: cfg.montecarlo_n.unwrap_or(2000) as usize,
        feasibility_tol: cfg.feasibility_tol.unwrap_or(1e-4),
        popsize: cfg.popsize.unwrap_or(10) as usize,
        max_iter: cfg.max_iter.unwrap_or(1000) as usize,
    }
}

fn hashmap_to_obj(map: &HashMap<String, f64>) -> JsValue {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    map.serialize(&serializer).unwrap_or(JsValue::NULL)
}

fn libertad_to_jsvec(entries: Vec<LibertadEntry>) -> Vec<JsValue> {
    entries
        .into_iter()
        .map(|e| {
            serde_wasm_bindgen::to_value(&e).unwrap_or(JsValue::NULL)
        })
        .collect()
}

#[wasm_bindgen]
pub fn validate(script: String, cfg: &JsSolverConfig) -> JsValidationResult {
    let config = to_solver_config(cfg);
    match crate::validate_dsl(&script, &config) {
        Ok(()) => JsValidationResult {
            valid: true,
            errors: Vec::new(),
        },
        Err(errors) => JsValidationResult {
            valid: false,
            errors,
        },
    }
}

#[wasm_bindgen]
pub fn solve(script: String, cfg: &JsSolverConfig) -> JsSolverResult {
    let config = to_solver_config(cfg);
    let result = crate::solve(&script, &config);

    JsSolverResult {
        feasible: result.feasible,
        plan: hashmap_to_obj(&result.plan),
        penalty: result.penalty,
        strategy: result.strategy,
        probability: result.probability,
        effectiveness: result.effectiveness,
        montecarlo_samples: result.montecarlo_samples as i32,
        constraint_violations: result.constraint_violations,
        libertad: libertad_to_jsvec(result.libertad),
        elapsed_ms: result.elapsed_ms as i32,
    }
}


