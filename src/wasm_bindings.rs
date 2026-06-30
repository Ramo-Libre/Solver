use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

use crate::*;

/// Inicializa el módulo WASM (se llama automáticamente al importar)
#[wasm_bindgen(start)]
pub fn init_wasm() {
    console_error_panic_hook::set_once();
}

/// Tipo que recibe la función solve desde JS (Frontend)
#[derive(Deserialize)]
pub struct JsSolverConfig {
    pub strategy: String,
    pub default_domain_lo: Option<f64>,
    pub default_domain_hi: Option<f64>,
    pub penalty_weight: Option<f64>,
    pub montecarlo_n: Option<i32>,
    pub feasibility_tol: Option<f64>,
    pub popsize: Option<i32>,
    pub max_iter: Option<i32>,
}

/// Entrada de libertad (slack) por restricción
#[derive(Serialize)]
pub struct JsLibertadEntry {
    pub label: Option<String>,
    pub raw: String,
    pub slack: f64,
    pub penalty: f64,
}

/// Resultado del solver expuesto a JS
#[derive(Serialize)]
pub struct JsSolverResult {
    pub feasible: bool,
    pub plan: HashMap<String, f64>,
    pub penalty: f64,
    pub strategy: String,
    pub probability: f64,
    pub effectiveness: f64,
    pub montecarlo_samples: i32,
    pub constraint_violations: Vec<String>,
    pub libertad: Vec<JsLibertadEntry>,
    pub elapsed_ms: i32,
}

fn to_solver_config(cfg: JsSolverConfig) -> SolverConfig {
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

/// Resultado de validación expuesto a JS
#[derive(Serialize)]
pub struct JsValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

/// Valida un script DSL sin ejecutar el solver.
/// Útil para feedback en tiempo real desde el frontend.
#[wasm_bindgen]
pub fn validate(script: String, cfg_val: JsValue) -> Result<JsValue, JsValue> {
    let cfg: JsSolverConfig = serde_wasm_bindgen::from_value(cfg_val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let config = to_solver_config(cfg);

    match crate::validate_dsl(&script, &config) {
        Ok(()) => serde_wasm_bindgen::to_value(&JsValidationResult {
            valid: true,
            errors: Vec::new(),
        }),
        Err(errors) => serde_wasm_bindgen::to_value(&JsValidationResult {
            valid: false,
            errors,
        }),
    }
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Resuelve un script DSL y retorna el resultado para la Web.
#[wasm_bindgen]
pub fn solve(script: String, cfg_val: JsValue) -> Result<JsValue, JsValue> {
    // 1. Deserializar el objeto de configuración que viene de JS
    let cfg: JsSolverConfig = serde_wasm_bindgen::from_value(cfg_val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let config = to_solver_config(cfg);
    let result = crate::solve(&script, &config);

    let js_res = JsSolverResult {
        feasible: result.feasible,
        plan: result.plan,
        penalty: result.penalty,
        strategy: result.strategy,
        probability: result.probability,
        effectiveness: result.effectiveness,
        montecarlo_samples: result.montecarlo_samples as i32,
        constraint_violations: result.constraint_violations,
        libertad: result
            .libertad
            .into_iter()
            .map(|e| JsLibertadEntry {
                label: e.label,
                raw: e.raw,
                slack: e.slack,
                penalty: e.penalty,
            })
            .collect(),
        elapsed_ms: result.elapsed_ms as i32,
    };

    // 2. Serializar el resultado final a un objeto JS nativo
    serde_wasm_bindgen::to_value(&js_res)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
