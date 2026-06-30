// ============================================================
// RamoLibre Solver — napi-rs bindings para TypeScript/Node.js
// ============================================================

use std::collections::HashMap;
use napi_derive::napi;

use crate::*;

/// Tipo que recibe la función solve desde JS
#[napi(object)]
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
#[napi(object)]
pub struct JsLibertadEntry {
    pub label: Option<String>,
    pub raw: String,
    pub slack: f64,
    pub penalty: f64,
}

/// Resultado del solver expuesto a JS
#[napi(object)]
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
            cfg.default_domain_lo.unwrap_or(1.0),
            cfg.default_domain_hi.unwrap_or(7.0),
        ),
        penalty_weight: cfg.penalty_weight.unwrap_or(1e6),
        montecarlo_n: cfg.montecarlo_n.unwrap_or(2000) as usize,
        feasibility_tol: cfg.feasibility_tol.unwrap_or(1e-4),
        popsize: cfg.popsize.unwrap_or(10) as usize,
        max_iter: cfg.max_iter.unwrap_or(1000) as usize,
    }
}

/// Resuelve un script DSL y retorna el resultado.
/// Equivalente a ejecutar el CLI con `--json`.
#[napi]
pub fn solve(script: String, cfg: JsSolverConfig) -> JsSolverResult {
    let config = to_solver_config(cfg);
    let result = crate::solve(&script, &config);

    JsSolverResult {
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
    }
}
