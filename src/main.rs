// ============================================================
// RamoLibre Solver — CLI
//
// Uso:
//   echo "NF = NP * 0.6 + Ex * 0.4 ..." | ramo_solver
//   ramo_solver < ramo.dsl
//   ramo_solver --strategy minimo_esfuerzo --montecarlo 3000 < ramo.dsl
//
// Flags:
//   --strategy   <nombre>   punto_medio | minimo_esfuerzo | maximo_seguridad | minimo_varianza
//   --montecarlo <n>        número de muestras Monte Carlo (default: 2000)
//   --popsize    <n>        tamaño de población DE por dimensión (default: 10)
//   --penalty    <f>        peso de penalización (default: 1e6)
//   --domain     <lo,hi>    dominio por defecto si no se especifica en el DSL (default: 1.0,7.0)
//   --json                  output en JSON (default)
//   --pretty                output legible para humanos
//   --help                  muestra esta ayuda
// ============================================================

use std::io::{self, Read};

use solver::*;

fn print_help() {
    eprintln!("RamoLibre Solver — Resuelve ecuaciones académicas no-lineales");
    eprintln!();
    eprintln!("USO:");
    eprintln!("  ramo_solver [opciones] < archivo.dsl");
    eprintln!("  echo 'NF = NP * 0.6 + Ex * 0.4\\nNF >= 4.0\\n...' | ramo_solver");
    eprintln!();
    eprintln!("OPCIONES:");
    eprintln!("  --strategy <nombre>   punto_medio | minimo_esfuerzo | maximo_seguridad | minimo_varianza");
    eprintln!("  --montecarlo <n>      muestras Monte Carlo (default: 2000)");
    eprintln!("  --popsize <n>         población DE por dimensión (default: 10)");
    eprintln!("  --penalty <f>         peso de penalización (default: 1000000)");
    eprintln!("  --domain <lo,hi>      dominio por defecto (default: 1.0,7.0)");
    eprintln!("  --pretty              output legible para humanos");
    eprintln!("  --feasibility-tol <f> tolerancia de factibilidad (default: 0.0001)");
    eprintln!("  --help                muestra esta ayuda");
    eprintln!();
    eprintln!("EJEMPLO DSL:");
    eprintln!("  // Ramo simple");
    eprintln!("  NF = NP * 0.6 + Ex * 0.4");
    eprintln!("  NF >= 4.0");
    eprintln!("  NP in [1.0, 7.0]");
    eprintln!("  Ex in [1.0, 7.0]");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Parsear flags
    let mut strategy = Strategy::PuntoMedio;
    let mut montecarlo_n: usize = 2000;
    let mut popsize: usize = 10;
    let mut penalty_weight: f64 = 1e6;
    let mut default_domain: (f64, f64) = (1.0, 7.0);
    let mut feasibility_tol: f64 = 1e-4;
    let mut pretty = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--pretty" => { pretty = true; }
            "--json"   => { pretty = false; }
            "--strategy" => {
                i += 1;
                if let Some(s) = args.get(i) {
                    strategy = Strategy::from_str(s);
                }
            }
            "--montecarlo" => {
                i += 1;
                if let Some(s) = args.get(i) {
                    montecarlo_n = s.parse().unwrap_or(2000);
                }
            }
            "--popsize" => {
                i += 1;
                if let Some(s) = args.get(i) {
                    popsize = s.parse().unwrap_or(10);
                }
            }
            "--penalty" => {
                i += 1;
                if let Some(s) = args.get(i) {
                    penalty_weight = s.parse().unwrap_or(1e6);
                }
            }
            "--domain" => {
                i += 1;
                if let Some(s) = args.get(i) {
                    let parts: Vec<f64> = s.split(',')
                        .filter_map(|p| p.trim().parse().ok())
                        .collect();
                    if parts.len() == 2 {
                        default_domain = (parts[0], parts[1]);
                    }
                }
            }
            "--feasibility-tol" => {
                i += 1;
                if let Some(s) = args.get(i) {
                    feasibility_tol = s.parse().unwrap_or(1e-4);
                }
            }
            unknown => {
                eprintln!("Flag desconocido: '{}'. Usa --help para ver opciones.", unknown);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    // Leer DSL desde stdin
    let mut script = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut script) {
        eprintln!("Error leyendo stdin: {}", e);
        std::process::exit(1);
    }

    if script.trim().is_empty() {
        eprintln!("Error: stdin vacío. Pasa el DSL por stdin.");
        eprintln!("Usa --help para ver ejemplos.");
        std::process::exit(1);
    }

    let config = SolverConfig {
        strategy,
        default_domain,
        penalty_weight,
        montecarlo_n,
        feasibility_tol,
        popsize,
        max_iter: 1000,
    };

    let result = solve(&script, &config);

    if pretty {
        // Output legible
        println!();
        println!("══════════════════════════════════════════");
        println!("  RamoLibre Solver  |  {}", result.strategy);
        println!("══════════════════════════════════════════");
        println!("  Factible       : {}", result.feasible);
        println!("  Penalización   : {}", result.penalty);
        println!("  Tiempo         : {}ms", result.elapsed_ms);
        println!("  Probabilidad   : {:.1}%", result.probability * 100.0);
        println!("  Efectividad    : {:.1}%", result.effectiveness * 100.0);
        println!("  Plan:");

        // Ordenar plan para output consistente
        let mut plan_vec: Vec<_> = result.plan.iter().collect();
        plan_vec.sort_by_key(|(k, _)| k.as_str());
        for (k, v) in &plan_vec {
            println!("    {:12} = {}", k, v);
        }

        if !result.constraint_violations.is_empty() {
            println!("  Violaciones:");
            for v in &result.constraint_violations {
                println!("    ✗  {}", v);
            }
        }
        if !result.libertad.is_empty() {
            println!("  Libertad (slack):");
            for entry in &result.libertad {
                let label_prefix = match &entry.label {
                    Some(lbl) => format!("[{}] ", lbl),
                    None => String::new(),
                };
                println!("    {}{:50} slack={:.6}  penalty={:.2e}", label_prefix, entry.raw, entry.slack, entry.penalty);
            }
        }
        println!();
    } else {
        // Output JSON (default) — fácil de parsear desde TypeScript
        match serde_json::to_string_pretty(&result) {
            Ok(json) => println!("{}", json),
            Err(e)   => {
                eprintln!("Error serializando resultado: {}", e);
                std::process::exit(1);
            }
        }
    }
}
