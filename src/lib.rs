// ============================================================
// RamoLibre Solver — Librería Rust
// Espejo del DSL TypeScript: parser, evaluador AST, solver DE+SLSQP
// ============================================================

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rand::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

// ============================================================
// SECCIÓN 1: AST
// ============================================================

#[derive(Debug, Clone)]
pub enum AstNode {
    Number(f64),
    Variable(String),
    Binary {
        op: BinOp,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    Parens(Box<AstNode>),
    Function {
        name: String,
        args: Vec<AstNode>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Pow,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelOp {
    Gte, Lte, Gt, Lt, Eq,
}

impl RelOp {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            ">=" => Some(Self::Gte),
            "<=" => Some(Self::Lte),
            ">"  => Some(Self::Gt),
            "<"  => Some(Self::Lt),
            "==" => Some(Self::Eq),
            _    => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    Assignment {
        lhs: String,
        expr: AstNode,
        raw: String,
        label: Option<String>,
    },
    Constraint {
        left: AstNode,
        op: RelOp,
        right: AstNode,
        raw: String,
        label: Option<String>,
    },
    Domain {
        variables: Vec<String>,
        min: f64,
        max: f64,
        raw: String,
    },
}

// ============================================================
// SECCIÓN 2: TOKENIZADOR
// ============================================================

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Number(f64),
    Id(String),
    Op(String),   // +, -, *, /, **, >=, <=, ==, >, <
    Comma,
    LParen,
    RParen,
    Colon,
    Eof,
}

fn tokenize(input: &str) -> Vec<TokenKind> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Espacios
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }

        // Números
        if chars[i].is_ascii_digit() || (chars[i] == '.' && i + 1 < chars.len() && chars[i+1].is_ascii_digit()) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let num: f64 = chars[start..i].iter().collect::<String>().parse().unwrap_or(0.0);
            tokens.push(TokenKind::Number(num));
            continue;
        }

        // Identificadores y keywords
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let id: String = chars[start..i].iter().collect();
            tokens.push(TokenKind::Id(id));
            continue;
        }

        // Operadores multi-caracter primero
        if i + 1 < chars.len() {
            let two: String = chars[i..i+2].iter().collect();
            if matches!(two.as_str(), ">=" | "<=" | "==" | "**") {
                tokens.push(TokenKind::Op(two));
                i += 2;
                continue;
            }
        }

        // Operadores y puntuación un caracter
        match chars[i] {
            '+' => { tokens.push(TokenKind::Op("+".into())); i += 1; }
            '-' => { tokens.push(TokenKind::Op("-".into())); i += 1; }
            '*' => { tokens.push(TokenKind::Op("*".into())); i += 1; }
            '/' => { tokens.push(TokenKind::Op("/".into())); i += 1; }
            '>' => { tokens.push(TokenKind::Op(">".into())); i += 1; }
            '<' => { tokens.push(TokenKind::Op("<".into())); i += 1; }
            '(' => { tokens.push(TokenKind::LParen); i += 1; }
            ')' => { tokens.push(TokenKind::RParen); i += 1; }
            ',' => { tokens.push(TokenKind::Comma); i += 1; }
            ':' => { tokens.push(TokenKind::Colon); i += 1; }
            _   => { i += 1; } // caracter desconocido → ignorar
        }
    }

    tokens.push(TokenKind::Eof);
    tokens
}

// ============================================================
// SECCIÓN 3: PARSER RECURSIVE DESCENT
// ============================================================

struct ExprParser {
    tokens: Vec<TokenKind>,
    pos: usize,
}

impl ExprParser {
    fn new(tokens: Vec<TokenKind>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &TokenKind {
        self.tokens.get(self.pos).unwrap_or(&TokenKind::Eof)
    }

    fn consume(&mut self) -> TokenKind {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(TokenKind::Eof);
        self.pos += 1;
        t
    }

    fn expect(&mut self, val: &str) -> Result<(), String> {
        match self.peek() {
            TokenKind::RParen if val == ")" => { self.consume(); Ok(()) }
            TokenKind::Op(s) if s == val   => { self.consume(); Ok(()) }
            other => Err(format!("Se esperaba '{}', se encontró '{:?}'", val, other)),
        }
    }

    fn parse_primary(&mut self) -> Result<AstNode, String> {
        match self.peek().clone() {
            TokenKind::Number(n) => {
                self.consume();
                Ok(AstNode::Number(n))
            }
            TokenKind::Id(name) => {
                self.consume();
                // ¿función?
                if self.peek() == &TokenKind::LParen {
                    self.consume();
                    let mut args = Vec::new();
                    if self.peek() != &TokenKind::RParen {
                        args.push(self.parse_expr()?);
                        while self.peek() == &TokenKind::Comma {
                            self.consume();
                            args.push(self.parse_expr()?);
                        }
                    }
                    self.expect(")")?;
                    Ok(AstNode::Function { name, args })
                } else {
                    Ok(AstNode::Variable(name))
                }
            }
            TokenKind::LParen => {
                self.consume();
                let expr = self.parse_expr()?;
                self.expect(")")?;
                Ok(AstNode::Parens(Box::new(expr)))
            }
            other => Err(format!("Token inesperado: {:?}", other)),
        }
    }

    fn parse_unary(&mut self) -> Result<AstNode, String> {
        if self.peek() == &TokenKind::Op("-".into()) {
            self.consume();
            let operand = self.parse_primary()?;
            Ok(AstNode::Binary {
                op: BinOp::Sub,
                left: Box::new(AstNode::Number(0.0)),
                right: Box::new(operand),
            })
        } else {
            self.parse_primary()
        }
    }

    fn parse_power(&mut self) -> Result<AstNode, String> {
        let left = self.parse_unary()?;
        if self.peek() == &TokenKind::Op("**".into()) {
            self.consume();
            let right = self.parse_power()?; // asociativo por la derecha
            Ok(AstNode::Binary { op: BinOp::Pow, left: Box::new(left), right: Box::new(right) })
        } else {
            Ok(left)
        }
    }

    fn parse_mul_div(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_power()?;
        loop {
            let op = match self.peek() {
                TokenKind::Op(s) if s == "*" => BinOp::Mul,
                TokenKind::Op(s) if s == "/" => BinOp::Div,
                _ => break,
            };
            self.consume();
            let right = self.parse_power()?;
            left = AstNode::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_add_sub(&mut self) -> Result<AstNode, String> {
        let mut left = self.parse_mul_div()?;
        loop {
            let op = match self.peek() {
                TokenKind::Op(s) if s == "+" => BinOp::Add,
                TokenKind::Op(s) if s == "-" => BinOp::Sub,
                _ => break,
            };
            self.consume();
            let right = self.parse_mul_div()?;
            left = AstNode::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    pub fn parse_expr(&mut self) -> Result<AstNode, String> {
        self.parse_add_sub()
    }
}

fn parse_dsl(src: &str) -> Result<AstNode, String> {
    let tokens = tokenize(src);
    let mut parser = ExprParser::new(tokens);
    parser.parse_expr()
}

// ============================================================
// SECCIÓN 4: PARSER DE STATEMENTS
// ============================================================

fn extract_label(line: &str) -> (Option<String>, &str) {
    if let Some(idx) = line.find(':') {
        // Asegurar que no es '::'
        let after = line.get(idx+1..idx+2);
        if after != Some(":") {
            let possible_label = line[..idx].trim();
            let rest = line[idx+1..].trim();
            // El label no puede contener operadores
            if !possible_label.is_empty()
                && !possible_label.contains(|c: char| "=<>+-*/()[]".contains(c))
            {
                return (Some(possible_label.to_string()), rest);
            }
        }
    }
    (None, line)
}

fn split_relational(line: &str) -> Option<(&str, &str, &str)> {
    // Orden importa: primero los de 2 chars para no confundir '>' con '>='
    for op in &[">=", "<=", "==", ">", "<"] {
        if let Some(idx) = line.find(op) {
            let left = line[..idx].trim();
            let right = line[idx + op.len()..].trim();
            if !left.is_empty() && !right.is_empty()
                && !left.contains(|c: char| "<>=".contains(c))
                && !right.contains(|c: char| "<>=".contains(c))
            {
                return Some((left, op, right));
            }
        }
    }
    None
}

fn parse_domain_stmt(rest: &str, raw: &str) -> Option<Statement> {
    // "dominio V1, V2 [min, max]"
    let rest_clean = rest.trim_start_matches(|c: char| c.is_alphabetic()); // quita "dominio"
    let rest_clean = rest_clean.trim();
    parse_domain_vars_range(rest_clean, raw)
}

fn parse_in_stmt(line: &str, raw: &str) -> Option<Statement> {
    // "V1, V2 in [min, max]"
    let in_idx = line.find(" in [")?;
    let vars_str = line[..in_idx].trim();
    let range_str = line[in_idx + 4..].trim(); // salta " in "
    let vars: Vec<String> = vars_str.split(',').map(|v| v.trim().to_string()).collect();
    parse_range(range_str).map(|(min, max)| Statement::Domain {
        variables: vars,
        min,
        max,
        raw: raw.to_string(),
    })
}

fn parse_domain_vars_range(s: &str, raw: &str) -> Option<Statement> {
    let bracket_idx = s.find('[')?;
    let vars_str = s[..bracket_idx].trim().trim_end_matches(',').trim();
    let range_str = s[bracket_idx..].trim();
    let vars: Vec<String> = vars_str.split(',').map(|v| v.trim().to_string()).collect();
    parse_range(range_str).map(|(min, max)| Statement::Domain {
        variables: vars,
        min,
        max,
        raw: raw.to_string(),
    })
}

fn parse_range(s: &str) -> Option<(f64, f64)> {
    // "[min, max]"
    let s = s.trim().strip_prefix('[')?.strip_suffix(']')?.trim();
    let mut parts = s.splitn(2, ',');
    let min: f64 = parts.next()?.trim().parse().ok()?;
    let max: f64 = parts.next()?.trim().parse().ok()?;
    Some((min, max))
}

pub fn parse_script(script: &str) -> Vec<Statement> {
    let mut stmts = Vec::new();

    for original_line in script.lines() {
        let line = original_line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if let Some(stmt) = parse_line(line) {
            stmts.push(stmt);
        }
    }

    stmts
}

fn parse_line(line: &str) -> Option<Statement> {
    let (label, rest) = extract_label(line);

    // Dominio con keyword "dominio"
    if rest.to_lowercase().starts_with("dominio") {
        return parse_domain_stmt(rest, line);
    }

    // Dominio con "in ["
    if rest.contains(" in [") {
        return parse_in_stmt(rest, line);
    }

    // Asignación: "VAR = expr" (donde expr no tiene operadores relacionales)
    let assign_re = rest.find('=')
        .filter(|&i| {
            let before = rest.get(i.saturating_sub(1)..i).unwrap_or("");
            let after = rest.get(i+1..i+2).unwrap_or("");
            before != ">" && before != "<" && before != "!" && after != "="
        });

    if let Some(eq_idx) = assign_re {
        let lhs = rest[..eq_idx].trim();
        let rhs = rest[eq_idx+1..].trim();
        // Verificar que lhs es un identificador simple
        let is_id = lhs.chars().all(|c| c.is_alphanumeric() || c == '_')
            && !lhs.is_empty()
            && lhs.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_');
        // Verificar que rhs no tiene operador relacional (no es una constraint)
        if is_id && split_relational(rhs).is_none() {
            if let Ok(expr) = parse_dsl(rhs) {
                return Some(Statement::Assignment {
                    lhs: lhs.to_string(),
                    expr,
                    raw: line.to_string(),
                    label,
                });
            }
        }
    }

    // Constraint: expr OP expr
    if let Some((left_str, op_str, right_str)) = split_relational(rest) {
        if let (Ok(left), Some(op), Ok(right)) = (
            parse_dsl(left_str),
            RelOp::from_str(op_str),
            parse_dsl(right_str),
        ) {
            return Some(Statement::Constraint {
                left,
                op,
                right,
                raw: line.to_string(),
                label,
            });
        }
    }

    None
}

// ============================================================
// SECCIÓN 5: EVALUADOR AST
// ============================================================

pub type Context = HashMap<String, f64>;

pub fn eval_ast(node: &AstNode, ctx: &Context) -> Result<f64, String> {
    match node {
        AstNode::Number(n) => Ok(*n),

        AstNode::Variable(name) => {
            ctx.get(name).copied()
                .ok_or_else(|| format!("Variable no definida: '{}'", name))
        }

        AstNode::Parens(inner) => eval_ast(inner, ctx),

        AstNode::Binary { op, left, right } => {
            let l = eval_ast(left, ctx)?;
            let r = eval_ast(right, ctx)?;
            match op {
                BinOp::Add => Ok(l + r),
                BinOp::Sub => Ok(l - r),
                BinOp::Mul => Ok(l * r),
                BinOp::Div => {
                    if r == 0.0 { Err("División por cero".into()) }
                    else { Ok(l / r) }
                }
                BinOp::Pow => Ok(l.powf(r)),
            }
        }

        AstNode::Function { name, args } => {
            let vals: Result<Vec<f64>, _> = args.iter().map(|a| eval_ast(a, ctx)).collect();
            let vals = vals?;
            match name.as_str() {
                "prom" => {
                    if vals.is_empty() { return Err("prom() requiere argumentos".into()); }
                    Ok(vals.iter().sum::<f64>() / vals.len() as f64)
                }
                "cada" => {
                    if vals.is_empty() { return Err("cada() requiere argumentos".into()); }
                    Ok(vals.iter().cloned().fold(f64::INFINITY, f64::min))
                }
                "escalon" => {
                    if vals.is_empty() { return Err("escalon() requiere un argumento".into()); }
                    Ok(if vals[0] >= 0.0 { 1.0 } else { 0.0 })
                }
                "min" => {
                    if vals.is_empty() { return Err("min() requiere argumentos".into()); }
                    Ok(vals.iter().cloned().fold(f64::INFINITY, f64::min))
                }
                "max" => {
                    if vals.is_empty() { return Err("max() requiere argumentos".into()); }
                    Ok(vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
                }
                _ => Err(format!("Función no implementada: '{}'", name)),
            }
        }
    }
}

pub fn build_context(stmts: &[Statement], free_vars: &HashMap<String, f64>) -> Context {
    let mut ctx: Context = free_vars.clone();

    let assignments: Vec<_> = stmts.iter().filter_map(|s| {
        if let Statement::Assignment { lhs, expr, .. } = s { Some((lhs, expr)) } else { None }
    }).collect();

    // Hasta 10 pasadas para resolver dependencias entre asignaciones
    for _ in 0..10 {
        let mut changed = false;
        for (lhs, expr) in &assignments {
            if let Ok(val) = eval_ast(expr, &ctx) {
                let prev = ctx.get(*lhs).copied();
                if prev != Some(val) {
                    ctx.insert((*lhs).clone(), val);
                    changed = true;
                }
            }
        }
        if !changed { break; }
    }

    ctx
}

// ============================================================
// SECCIÓN 6: EXTRACTORES DE METADATOS
// ============================================================

pub fn extract_calculated_vars(stmts: &[Statement]) -> std::collections::HashSet<String> {
    stmts.iter().filter_map(|s| {
        if let Statement::Assignment { lhs, .. } = s { Some(lhs.clone()) } else { None }
    }).collect()
}

fn collect_vars_from_ast(node: &AstNode, found: &mut std::collections::HashSet<String>) {
    match node {
        AstNode::Variable(name) => { found.insert(name.clone()); }
        AstNode::Parens(inner)  => collect_vars_from_ast(inner, found),
        AstNode::Binary { left, right, .. } => {
            collect_vars_from_ast(left, found);
            collect_vars_from_ast(right, found);
        }
        AstNode::Function { args, .. } => {
            for a in args { collect_vars_from_ast(a, found); }
        }
        AstNode::Number(_) => {}
    }
}

pub fn extract_free_variables(stmts: &[Statement]) -> Vec<String> {
    let calc = extract_calculated_vars(stmts);
    let mut found = std::collections::HashSet::new();

    for s in stmts {
        match s {
            Statement::Assignment { expr, .. } => collect_vars_from_ast(expr, &mut found),
            Statement::Constraint { left, right, .. } => {
                collect_vars_from_ast(left, &mut found);
                collect_vars_from_ast(right, &mut found);
            }
            _ => {}
        }
    }

    let mut vars: Vec<String> = found.into_iter().filter(|v| !calc.contains(v)).collect();
    vars.sort(); // orden determinista
    vars
}

pub fn extract_domains(stmts: &[Statement]) -> HashMap<String, (f64, f64)> {
    let mut domains = HashMap::new();
    for s in stmts {
        if let Statement::Domain { variables, min, max, .. } = s {
            for v in variables {
                domains.insert(v.clone(), (*min, *max));
            }
        }
    }
    domains
}

pub fn extract_constraints(stmts: &[Statement]) -> Vec<&Statement> {
    stmts.iter().filter(|s| matches!(s, Statement::Constraint { .. })).collect()
}

// ============================================================
// SECCIÓN 7: SOLVER
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibertadEntry {
    pub label: Option<String>,
    pub raw: String,
    pub slack: f64,
    pub penalty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverResult {
    pub feasible: bool,
    pub plan: HashMap<String, f64>,
    pub penalty: f64,
    pub strategy: String,
    pub probability: f64,
    pub effectiveness: f64,
    pub montecarlo_samples: usize,
    pub constraint_violations: Vec<String>,
    pub libertad: Vec<LibertadEntry>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    pub strategy: Strategy,
    pub default_domain: (f64, f64),
    pub penalty_weight: f64,
    pub montecarlo_n: usize,
    pub feasibility_tol: f64,
    pub popsize: usize,
    pub max_iter: usize,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            strategy: Strategy::PuntoMedio,
            default_domain: (1.0, 7.0),
            penalty_weight: 1e6,
            montecarlo_n: 2000,
            feasibility_tol: 1e-4,
            popsize: 10,
            max_iter: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    MinimoEsfuerzo,
    MaximoSeguridad,
    PuntoMedio,
    MinimoVarianza,
}

impl Strategy {
    pub fn from_str(s: &str) -> Self {
        match s {
            "minimo_esfuerzo"  => Self::MinimoEsfuerzo,
            "maximo_seguridad" => Self::MaximoSeguridad,
            "minimo_varianza"  => Self::MinimoVarianza,
            _                  => Self::PuntoMedio,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::MinimoEsfuerzo  => "minimo_esfuerzo",
            Self::MaximoSeguridad => "maximo_seguridad",
            Self::PuntoMedio      => "punto_medio",
            Self::MinimoVarianza  => "minimo_varianza",
        }
    }

    /// Objetivo secundario que desempata entre soluciones igualmente factibles
    fn secondary_objective(&self, x: &[f64], mids: &[f64]) -> f64 {
        match self {
            Self::MinimoEsfuerzo  => x.iter().sum::<f64>(),
            Self::MaximoSeguridad => -x.iter().sum::<f64>(),
            Self::PuntoMedio      => x.iter().zip(mids).map(|(xi, m)| (xi - m).powi(2)).sum(),
            Self::MinimoVarianza  => {
                let mean = x.iter().sum::<f64>() / x.len() as f64;
                x.iter().map(|xi| (xi - mean).powi(2)).sum()
            }
        }
    }
}

/// Penalización de una constraint individual (0 si se satisface)
fn constraint_penalty(stmt: &Statement, ctx: &Context) -> f64 {
    if let Statement::Constraint { left, op, right, .. } = stmt {
        let l = eval_ast(left, ctx).unwrap_or(f64::NAN);
        let r = eval_ast(right, ctx).unwrap_or(f64::NAN);
        if l.is_nan() || r.is_nan() { return 1e6; }
        match op {
            RelOp::Gte => f64::max(0.0, r - l),
            RelOp::Lte => f64::max(0.0, l - r),
            RelOp::Gt  => f64::max(0.0, r - l + 1e-9),
            RelOp::Lt  => f64::max(0.0, l - r + 1e-9),
            RelOp::Eq  => (l - r).abs(),
        }
    } else {
        0.0
    }
}

fn total_penalty(stmts: &[Statement], ctx: &Context) -> f64 {
    stmts.iter()
        .map(|s| constraint_penalty(s, ctx).powi(2))
        .sum()
}

/// Función objetivo = penalización fuerte + objetivo de estrategia
fn objective(
    x: &[f64],
    free_var_names: &[String],
    stmts: &[Statement],
    mids: &[f64],
    strategy: &Strategy,
    penalty_weight: f64,
) -> f64 {
    let free: HashMap<String, f64> = free_var_names.iter()
        .zip(x.iter())
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    let ctx = build_context(stmts, &free);

    let pen: f64 = stmts.iter()
        .map(|s| constraint_penalty(s, &ctx).powi(2))
        .sum();

    penalty_weight * pen + strategy.secondary_objective(x, mids)
}

// ── Differential Evolution ────────────────────────────────────────────────────

fn differential_evolution(
    stmts: &[Statement],
    free_var_names: &[String],
    bounds: &[(f64, f64)],
    mids: &[f64],
    strategy: &Strategy,
    penalty_weight: f64,
    popsize: usize,
    max_iter: usize,
    seed: u64,
) -> Vec<f64> {
    let n = bounds.len();
    let pop_size = popsize * n;
    let mut rng = SmallRng::seed_from_u64(seed);

    // Inicializar población aleatoria dentro de los bounds
    let mut pop: Vec<Vec<f64>> = (0..pop_size).map(|_| {
        bounds.iter().map(|(lo, hi)| rng.random_range(*lo..=*hi)).collect()
    }).collect();

    let mut fitness: Vec<f64> = pop.iter().map(|x| {
        objective(x, free_var_names, stmts, mids, strategy, penalty_weight)
    }).collect();

    let f_scale = 0.8;  // mutation factor
    let cr = 0.9;       // crossover rate

    for _ in 0..max_iter {
        let mut improved = false;

        for i in 0..pop_size {
            // Seleccionar 3 individuos distintos (a, b, c)
            let mut idxs = [0usize; 3];
            let mut count = 0;
            while count < 3 {
                let r = rng.random_range(0..pop_size);
                if r != i && !idxs[..count].contains(&r) {
                    idxs[count] = r;
                    count += 1;
                }
            }
            let (a, b, c) = (idxs[0], idxs[1], idxs[2]);

            // Mutación: v = pop[a] + F * (pop[b] - pop[c])
            let mutant: Vec<f64> = (0..n).map(|j| {
                let v = pop[a][j] + f_scale * (pop[b][j] - pop[c][j]);
                v.clamp(bounds[j].0, bounds[j].1)
            }).collect();

            // Crossover binomial
            let j_rand = rng.random_range(0..n);
            let trial: Vec<f64> = (0..n).map(|j| {
                if rng.random::<f64>() < cr || j == j_rand { mutant[j] } else { pop[i][j] }
            }).collect();

            // Selección
            let trial_fit = objective(&trial, free_var_names, stmts, mids, strategy, penalty_weight);
            if trial_fit <= fitness[i] {
                pop[i] = trial;
                fitness[i] = trial_fit;
                improved = true;
            }
        }

        if !improved { break; }
    }

    // Retornar el mejor individuo
    let best_idx = fitness.iter().enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);

    pop[best_idx].clone()
}

// ── SLSQP simplificado: descenso por gradiente numérico con proyección ────────

fn slsqp_polish(
    x0: Vec<f64>,
    free_var_names: &[String],
    stmts: &[Statement],
    bounds: &[(f64, f64)],
    mids: &[f64],
    strategy: &Strategy,
    penalty_weight: f64,
    max_iter: usize,
) -> Vec<f64> {
    let n = x0.len();
    let eps = 1e-7_f64;
    let mut x = x0;
    let mut step = 0.1_f64;

    let obj = |xv: &[f64]| objective(xv, free_var_names, stmts, mids, strategy, penalty_weight);

    for _ in 0..max_iter {
        let f0 = obj(&x);

        // Gradiente numérico
        let mut grad = vec![0.0_f64; n];
        for j in 0..n {
            let mut x_plus = x.clone();
            x_plus[j] += eps;
            x_plus[j] = x_plus[j].clamp(bounds[j].0, bounds[j].1);
            grad[j] = (obj(&x_plus) - f0) / eps;
        }

        // Paso en dirección del gradiente negativo
        let grad_norm: f64 = grad.iter().map(|g| g * g).sum::<f64>().sqrt();
        if grad_norm < 1e-12 { break; }

        let mut x_new: Vec<f64> = x.iter().zip(grad.iter())
            .map(|(xi, gi)| xi - step * gi / grad_norm)
            .collect();

        // Proyección a bounds
        for j in 0..n {
            x_new[j] = x_new[j].clamp(bounds[j].0, bounds[j].1);
        }

        let f_new = obj(&x_new);

        if f_new < f0 {
            x = x_new;
            step *= 1.1; // ampliar paso si mejora
        } else {
            step *= 0.5; // reducir paso si no mejora
            if step < 1e-14 { break; }
        }
    }

    x
}

// ── Entrada pública del solver ────────────────────────────────────────────────

pub fn solve(script: &str, config: &SolverConfig) -> SolverResult {
    let start = Instant::now();

    let stmts = parse_script(script);
    let free_vars = extract_free_variables(&stmts);
    let domains = extract_domains(&stmts);

    let bounds: Vec<(f64, f64)> = free_vars.iter()
        .map(|v| *domains.get(v).unwrap_or(&config.default_domain))
        .collect();

    let mids: Vec<f64> = bounds.iter().map(|(lo, hi)| (lo + hi) / 2.0).collect();

    // ── Paso 1: Differential Evolution (búsqueda global) ─────────────────────
    let x_de = differential_evolution(
        &stmts, &free_vars, &bounds, &mids,
        &config.strategy, config.penalty_weight,
        config.popsize, config.max_iter, 42,
    );

    // ── Paso 2: Polish con descenso de gradiente proyectado ───────────────────
    let x_polished = slsqp_polish(
        x_de.clone(), &free_vars, &stmts, &bounds, &mids,
        &config.strategy, config.penalty_weight, 5000,
    );

    // Elegir el mejor según penalización real
    let pen_de = {
        let free: HashMap<_, _> = free_vars.iter().zip(x_de.iter()).map(|(k,v)|(k.clone(),*v)).collect();
        let ctx = build_context(&stmts, &free);
        total_penalty(&stmts, &ctx)
    };
    let pen_pol = {
        let free: HashMap<_, _> = free_vars.iter().zip(x_polished.iter()).map(|(k,v)|(k.clone(),*v)).collect();
        let ctx = build_context(&stmts, &free);
        total_penalty(&stmts, &ctx)
    };

    let x_best = if pen_pol <= pen_de { x_polished } else { x_de };

    // ── Evaluar resultado ─────────────────────────────────────────────────────
    let free_dict: HashMap<String, f64> = free_vars.iter()
        .zip(x_best.iter())
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    let ctx_best = build_context(&stmts, &free_dict);
    let final_pen = total_penalty(&stmts, &ctx_best);
    let feasible = final_pen < config.feasibility_tol;

    let violations: Vec<String> = stmts.iter()
        .filter(|s| matches!(s, Statement::Constraint { .. }))
        .filter(|s| constraint_penalty(s, &ctx_best) > config.feasibility_tol)
        .filter_map(|s| match s {
            Statement::Constraint { raw, .. } => Some(raw.clone()),
            _ => None,
        })
        .collect();

    // ── Libertad (slack) de cada restricción ──────────────────────────────────
    let libertad: Vec<LibertadEntry> = stmts.iter()
        .filter_map(|s| {
            if let Statement::Constraint { left, op, right, raw, label } = s {
                let l = eval_ast(left, &ctx_best).unwrap_or(f64::NAN);
                let r = eval_ast(right, &ctx_best).unwrap_or(f64::NAN);
                let raw_penalty = constraint_penalty(s, &ctx_best);
                let penalty = if raw_penalty < config.feasibility_tol { 0.0 } else { raw_penalty };
                let slack = if l.is_nan() || r.is_nan() {
                    None
                } else {
                    Some(match op {
                        RelOp::Gte => l - r,
                        RelOp::Lte => r - l,
                        RelOp::Gt  => l - r,
                        RelOp::Lt  => r - l,
                        RelOp::Eq  => -(l - r).abs(),
                    })
                };
                let slack = slack.map(|v| if v.abs() < config.feasibility_tol { 0.0 } else { v });
                Some(LibertadEntry {
                    label: label.clone(),
                    raw: raw.clone(),
                    slack: slack.unwrap_or(f64::NAN),
                    penalty,
                })
            } else {
                None
            }
        })
        .collect();

    // ── Monte Carlo ───────────────────────────────────────────────────────────
    let mut rng = SmallRng::seed_from_u64(0);
    let n_vars = bounds.len();
    let mut feasible_count = 0usize;
    let mut near_plan_count = 0usize;
    let radii: Vec<f64> = bounds.iter().map(|(lo, hi)| (hi - lo) * 0.20).collect();

    for _ in 0..config.montecarlo_n {
        let sample: Vec<f64> = bounds.iter()
            .map(|(lo, hi)| rng.random_range(*lo..=*hi))
            .collect();

        let free_s: HashMap<_, _> = free_vars.iter().zip(sample.iter()).map(|(k,v)|(k.clone(),*v)).collect();
        let ctx_s = build_context(&stmts, &free_s);
        let pen_s = total_penalty(&stmts, &ctx_s);

        if pen_s < config.feasibility_tol {
            feasible_count += 1;
            // ¿Está cerca del plan encontrado?
            if feasible && (0..n_vars).all(|j| (sample[j] - x_best[j]).abs() <= radii[j]) {
                near_plan_count += 1;
            }
        }
    }

    let probability = feasible_count as f64 / config.montecarlo_n as f64;
    let effectiveness = if feasible_count > 0 && feasible {
        near_plan_count as f64 / feasible_count as f64
    } else {
        0.0
    };

    // Plan final: variables libres + calculadas, redondeadas a 4 decimales
    let calc_vars = extract_calculated_vars(&stmts);
    let mut plan = HashMap::new();
    for (k, v) in &ctx_best {
        if free_dict.contains_key(k) || calc_vars.contains(k) {
            plan.insert(k.clone(), (v * 10000.0).round() / 10000.0);
        }
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;

    SolverResult {
        feasible,
        plan,
        penalty: (final_pen * 1e10).round() / 1e10,
        strategy: config.strategy.name().to_string(),
        probability: (probability * 10000.0).round() / 10000.0,
        effectiveness: (effectiveness * 10000.0).round() / 10000.0,
        montecarlo_samples: config.montecarlo_n,
        constraint_violations: violations,
        libertad,
        elapsed_ms,
    }
}

#[cfg(feature = "napi")]
pub mod binding;

#[cfg(feature = "wasm")]
pub mod wasm_bindings;
