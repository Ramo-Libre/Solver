#[cfg(feature = "wasm")]
extern crate solver;

#[cfg(feature = "wasm")]
#[test]
fn test_all_assigned_variables() {
    let config = solver::SolverConfig::default();
    let result = solver::solve("x = 5\nx > 0", &config);
    assert!(result.feasible, "Debería ser feasible: x=5, x>0 se cumple");
    assert_eq!(result.plan.get("x"), Some(&5.0));
    assert!(result.constraint_violations.is_empty());
}

#[cfg(feature = "wasm")]
#[test]
fn test_all_assigned_multiple_vars() {
    let config = solver::SolverConfig::default();
    let result = solver::solve("x = 10\ny = x / 2\ny > 0", &config);
    assert!(result.feasible, "Debería ser feasible: x=10, y=5, y>0 se cumple");
    assert_eq!(result.plan.get("x"), Some(&10.0));
    assert_eq!(result.plan.get("y"), Some(&5.0));
}

#[cfg(feature = "wasm")]
#[test]
fn test_all_assigned_infeasible() {
    let config = solver::SolverConfig::default();
    let result = solver::solve("x = 1\nx > 0\nx < 0", &config);
    assert!(!result.feasible, "x=1, x<0 es contradictory");
}
