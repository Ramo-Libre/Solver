"""
RamoLibre — Solver Prototipo (Python/NumPy)
==========================================
Replica el evaluador AST del DSL TypeScript y lo conecta a scipy.optimize
para resolver el CSP como problema de optimización con penalización.

Flujo:
  DSL string → parseScript() → [StatementNode] → solve() → SolverResult
"""

from __future__ import annotations
import math
import json
import re
from dataclasses import dataclass, field
from typing import Any
import numpy as np
from scipy.optimize import differential_evolution, minimize

# =============================================================================
# SECCIÓN 1: AST (espejo del TypeScript)
# =============================================================================

@dataclass
class NumberNode:
    type: str = "number"
    value: float = 0.0

@dataclass
class VariableNode:
    type: str = "variable"
    name: str = ""

@dataclass
class BinaryNode:
    type: str = "binary"
    operator: str = "+"   # +, -, *, /, **
    left: Any = None
    right: Any = None

@dataclass
class ParensNode:
    type: str = "parens"
    expression: Any = None

@dataclass
class FunctionNode:
    type: str = "function"
    name: str = ""
    args: list = field(default_factory=list)

ASTNode = NumberNode | VariableNode | BinaryNode | ParensNode | FunctionNode

@dataclass
class AssignmentStatement:
    type: str = "assignment"
    lhs: str = ""
    expression: Any = None
    raw: str = ""
    label: str | None = None

@dataclass
class ConstraintStatement:
    type: str = "constraint"
    left: Any = None
    operator: str = ">="
    right: Any = None
    raw: str = ""
    label: str | None = None

@dataclass
class DomainStatement:
    type: str = "domain"
    variables: list[str] = field(default_factory=list)
    min: float = 0.0
    max: float = 7.0
    raw: str = ""
    label: str | None = None

StatementNode = AssignmentStatement | ConstraintStatement | DomainStatement

# =============================================================================
# SECCIÓN 2: TOKENIZADOR
# =============================================================================

TOKEN_RE = re.compile(
    r'(\d+(?:\.\d+)?)'           # number
    r'|(\*\*|>=|<=|==|>|<|[+\-*/])'  # operator
    r'|([A-Za-z_]\w*)'           # identifier
    r'|(,)|(\()|(\))|(:)'         # punctuation
)

def tokenize(src: str) -> list[dict]:
    tokens = []
    for m in TOKEN_RE.finditer(src.strip()):
        num, op, ident, comma, lp, rp, colon = m.groups()
        if num is not None:
            tokens.append({"type": "number", "value": num})
        elif op is not None:
            tokens.append({"type": "op", "value": op})
        elif ident is not None:
            tokens.append({"type": "id", "value": ident})
        elif comma:
            tokens.append({"type": "comma", "value": ","})
        elif lp:
            tokens.append({"type": "lparen", "value": "("})
        elif rp:
            tokens.append({"type": "rparen", "value": ")"})
        elif colon:
            tokens.append({"type": "colon", "value": ":"})
    tokens.append({"type": "eof", "value": ""})
    return tokens

# =============================================================================
# SECCIÓN 3: PARSER RECURSIVE DESCENT
# =============================================================================

class ExprParser:
    def __init__(self, tokens):
        self.tokens = tokens
        self.pos = 0

    def peek(self):
        return self.tokens[self.pos] if self.pos < len(self.tokens) else {"type": "eof", "value": ""}

    def consume(self):
        t = self.peek()
        self.pos += 1
        return t

    def expect(self, value):
        t = self.peek()
        if t["value"] != value:
            raise SyntaxError(f"Se esperaba '{value}', se encontró '{t['value']}'")
        return self.consume()

    def parse_primary(self):
        t = self.peek()
        if t["type"] == "number":
            self.consume()
            return NumberNode(value=float(t["value"]))
        if t["type"] == "id":
            name = self.consume()["value"]
            if self.peek()["type"] == "lparen":
                self.consume()
                args = []
                if self.peek()["type"] != "rparen":
                    args.append(self.parse_expr())
                    while self.peek()["type"] == "comma":
                        self.consume()
                        args.append(self.parse_expr())
                self.expect(")")
                return FunctionNode(name=name, args=args)
            return VariableNode(name=name)
        if t["type"] == "lparen":
            self.consume()
            expr = self.parse_expr()
            self.expect(")")
            return ParensNode(expression=expr)
        raise SyntaxError(f"Token inesperado: '{t['value']}'")

    def parse_unary(self):
        if self.peek()["type"] == "op" and self.peek()["value"] == "-":
            self.consume()
            return BinaryNode(operator="-", left=NumberNode(value=0.0), right=self.parse_primary())
        return self.parse_primary()

    def parse_power(self):
        left = self.parse_unary()
        if self.peek()["type"] == "op" and self.peek()["value"] == "**":
            self.consume()
            right = self.parse_power()
            return BinaryNode(operator="**", left=left, right=right)
        return left

    def parse_mul_div(self):
        left = self.parse_power()
        while self.peek()["type"] == "op" and self.peek()["value"] in ("*", "/"):
            op = self.consume()["value"]
            right = self.parse_power()
            left = BinaryNode(operator=op, left=left, right=right)
        return left

    def parse_add_sub(self):
        left = self.parse_mul_div()
        while self.peek()["type"] == "op" and self.peek()["value"] in ("+", "-"):
            op = self.consume()["value"]
            right = self.parse_mul_div()
            left = BinaryNode(operator=op, left=left, right=right)
        return left

    def parse_expr(self):
        return self.parse_add_sub()


def parse_dsl(src: str) -> ASTNode:
    tokens = tokenize(src)
    return ExprParser(tokens).parse_expr()


RELATIONAL_OPS = (">=", "<=", "==", ">", "<")

def split_on_relational(line: str):
    m = re.search(r'>=|<=|==|>|<', line)
    if not m:
        return None
    op = m.group(0)
    left = line[:m.start()].strip()
    right = line[m.end():].strip()
    if not left or not right:
        return None
    if re.search(r'[<>=]', left) or re.search(r'[<>=]', right):
        return None
    return left, op, right

def extract_label(line: str):
    idx = line.find(":")
    if idx != -1 and (idx + 1 >= len(line) or line[idx + 1] != ":"):
        possible_label = line[:idx].strip()
        rest = line[idx + 1:].strip()
        if possible_label and not re.search(r'[=<>+\-*/()[\]]', possible_label):
            return possible_label, rest
    return None, line

def parse_line(line: str) -> StatementNode | None:
    label, rest = extract_label(line)

    if re.match(r'^dominio\b', rest, re.IGNORECASE):
        m = re.match(
            r'^dominio\s+([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)\s*\[\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)\s*\]$',
            rest, re.IGNORECASE
        )
        if not m:
            raise SyntaxError(f"Dominio inválido: {rest}")
        variables = [v.strip() for v in m.group(1).split(",")]
        return DomainStatement(variables=variables, min=float(m.group(2)), max=float(m.group(3)), raw=line, label=label)

    m_in = re.match(
        r'^([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)\s+in\s*\[\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)\s*\]$',
        rest, re.IGNORECASE
    )
    if m_in:
        variables = [v.strip() for v in m_in.group(1).split(",")]
        return DomainStatement(variables=variables, min=float(m_in.group(2)), max=float(m_in.group(3)), raw=line, label=label)

    assign_m = re.match(r'^([A-Za-z_]\w*)\s*=(?!=)\s*(.+)$', rest)
    if assign_m:
        lhs = assign_m.group(1).strip()
        rhs = assign_m.group(2).strip()
        if not split_on_relational(rhs):
            return AssignmentStatement(lhs=lhs, expression=parse_dsl(rhs), raw=line, label=label)

    parts = split_on_relational(rest)
    if parts:
        l_str, op, r_str = parts
        return ConstraintStatement(left=parse_dsl(l_str), operator=op, right=parse_dsl(r_str), raw=line, label=label)

    return None

def parse_script(script: str) -> list[StatementNode]:
    if not script.strip():
        return []
    statements = []
    for line in script.split("\n"):
        line = line.strip()
        if not line or line.startswith("//"):
            continue
        try:
            stmt = parse_line(line)
            if stmt:
                statements.append(stmt)
        except Exception:
            pass
    return statements

# =============================================================================
# SECCIÓN 4: EVALUADOR AST
# =============================================================================

def eval_ast(node: ASTNode, ctx: dict[str, float]) -> float:
    """Evalúa un nodo AST con el contexto dado. Espejo de evaluateASTDecimal."""
    t = node.type
    if t == "number":
        return node.value
    if t == "variable":
        if node.name not in ctx:
            raise KeyError(f"Variable no definida: {node.name}")
        return ctx[node.name]
    if t == "parens":
        return eval_ast(node.expression, ctx)
    if t == "binary":
        l = eval_ast(node.left, ctx)
        r = eval_ast(node.right, ctx)
        if node.operator == "+":  return l + r
        if node.operator == "-":  return l - r
        if node.operator == "*":  return l * r
        if node.operator == "/":
            if r == 0: raise ZeroDivisionError
            return l / r
        if node.operator == "**": return l ** r
    if t == "function":
        args = [eval_ast(a, ctx) for a in node.args]
        name = node.name
        if name == "prom":
            return sum(args) / len(args)
        if name == "cada":
            return min(args)
        if name == "escalon":
            return 1.0 if args[0] >= 0 else 0.0
        if name == "min":
            return min(args)
        if name == "max":
            return max(args)
        raise ValueError(f"Función no implementada: {name}")
    raise ValueError(f"Nodo desconocido: {t}")

def build_context(
    statements: list[StatementNode],
    free_vars: dict[str, float]
) -> dict[str, float]:
    """Construye el contexto propagando las asignaciones (≤ 10 pasadas)."""
    ctx = dict(free_vars)
    assignments = [s for s in statements if s.type == "assignment"]
    for _ in range(10):
        changed = False
        for stmt in assignments:
            try:
                val = eval_ast(stmt.expression, ctx)
                if ctx.get(stmt.lhs) != val:
                    ctx[stmt.lhs] = val
                    changed = True
            except Exception:
                pass
        if not changed:
            break
    return ctx

# =============================================================================
# SECCIÓN 5: EXTRACTOR DE METADATOS
# =============================================================================

def extract_calculated_vars(statements) -> set[str]:
    return {s.lhs for s in statements if s.type == "assignment"}

def collect_vars_from_ast(node, found: set):
    if node.type == "variable":
        found.add(node.name)
    elif node.type == "parens":
        collect_vars_from_ast(node.expression, found)
    elif node.type == "binary":
        collect_vars_from_ast(node.left, found)
        collect_vars_from_ast(node.right, found)
    elif node.type == "function":
        for a in node.args:
            collect_vars_from_ast(a, found)

def extract_free_variables(statements) -> list[str]:
    calc = extract_calculated_vars(statements)
    found = set()
    for s in statements:
        if s.type == "assignment":
            collect_vars_from_ast(s.expression, found)
        elif s.type == "constraint":
            collect_vars_from_ast(s.left, found)
            collect_vars_from_ast(s.right, found)
    return [v for v in found if v not in calc]

def extract_domains(statements) -> dict[str, tuple[float, float]]:
    domains = {}
    for s in statements:
        if s.type == "domain":
            for v in s.variables:
                domains[v] = (s.min, s.max)
    return domains

def extract_constraints(statements) -> list[ConstraintStatement]:
    return [s for s in statements if s.type == "constraint"]

# =============================================================================
# SECCIÓN 6: SOLVER
# =============================================================================

# Estrategias predefinidas — determinan el objetivo secundario
# Después del check de factibilidad (penalización → 0), el objetivo
# desempata entre soluciones igualmente factibles.
STRATEGIES = {
    "minimo_esfuerzo": lambda x, mid: np.sum(np.square(x - mid) * 0.0) + np.sum(x),         # minimiza sum(xi)
    "maximo_seguridad": lambda x, mid: -np.sum(x),                                            # maximiza sum(xi)
    "punto_medio":      lambda x, mid: np.sum(np.square(x - mid)),                            # cerca del mid
    "minimo_varianza":  lambda x, mid: np.var(x),                                             # todas iguales
}

@dataclass
class SolverResult:
    feasible: bool
    plan: dict[str, float]         # variables libres + calculadas
    penalty: float                  # 0 = perfectamente factible
    strategy: str
    probability: float              # fracción de muestras Monte Carlo que son factibles
    effectiveness: float            # entre las factibles, fracción que están cerca del plan
    montecarlo_samples: int
    constraint_violations: list[str]
    libertad: list[dict]            # slack por restricción: {"label": ..., "raw": ..., "slack": ..., "penalty": ...}

def _constraint_penalty(stmt: ConstraintStatement, ctx: dict[str, float]) -> float:
    """Retorna la violación numérica de una restricción (0 si se satisface)."""
    try:
        l = eval_ast(stmt.left, ctx)
        r = eval_ast(stmt.right, ctx)
        op = stmt.operator
        if op == ">=": return max(0.0, r - l)
        if op == "<=": return max(0.0, l - r)
        if op == ">":  return max(0.0, r - l + 1e-9)
        if op == "<":  return max(0.0, l - r + 1e-9)
        if op == "==": return abs(l - r)
    except Exception:
        return 1e6
    return 0.0

def make_objective(
    statements: list[StatementNode],
    free_var_names: list[str],
    bounds: list[tuple[float, float]],
    strategy: str,
    penalty_weight: float = 1e4,
):
    """
    Fábrica de la función objetivo para scipy.

    f(x) = penalty_weight * sum(violaciones²)   ← lleva al CSP
           + estrategia(x)                        ← desempata
    """
    constraints = extract_constraints(statements)
    mids = np.array([(lo + hi) / 2 for lo, hi in bounds])
    strat_fn = STRATEGIES.get(strategy, STRATEGIES["punto_medio"])

    def objective(x: np.ndarray) -> float:
        free = dict(zip(free_var_names, x))
        ctx = build_context(statements, free)

        pen = 0.0
        for c in constraints:
            v = _constraint_penalty(c, ctx)
            pen += v ** 2

        return penalty_weight * pen + strat_fn(x, mids)

    return objective

def solve(
    script: str,
    strategy: str = "punto_medio",
    default_domain: tuple[float, float] = (1.0, 7.0),
    penalty_weight: float = 1e6,
    montecarlo_n: int = 2000,
    feasibility_tol: float = 1e-4,
    polish: bool = True,
    popsize: int = 10,
) -> SolverResult:
    """
    Resuelve el CSP descrito en `script`.

    1. Parsea el DSL → StatementNode[]
    2. Extrae variables libres y dominios
    3. Differential Evolution global → punto inicial robusto
    4. SLSQP local para refinar (si polish=True)
    5. Monte Carlo para estimar probabilidad y efectividad
    """
    statements = parse_script(script)
    free_vars  = extract_free_variables(statements)
    domains    = extract_domains(statements)
    constraints = extract_constraints(statements)

    bounds = [domains.get(v, default_domain) for v in free_vars]

    # ── Paso 1: optimización global (Differential Evolution) ──────────────────
    objective = make_objective(statements, free_vars, bounds, strategy, penalty_weight)

    de_result = differential_evolution(
        objective,
        bounds=bounds,
        seed=42,
        maxiter=1000,
        tol=1e-8,
        mutation=(0.5, 1.5),
        recombination=0.9,
        popsize=popsize,
        workers=1,
    )
    x_best = de_result.x

    # ── Paso 2: refinamiento local SLSQP ──────────────────────────────────────
    if polish:
        # Cada constraint se evalúa a través del ctx completo (incluye vars calculadas)
        # así SLSQP ve la restricción en términos de las variables libres correctamente.
        scipy_constraints = []
        for c in constraints:
            def make_fn(stmt, names, stmts):
                def fn(x):
                    free = dict(zip(names, x))
                    ctx = build_context(stmts, free)
                    try:
                        l = eval_ast(stmt.left, ctx)
                        r = eval_ast(stmt.right, ctx)
                        op = stmt.operator
                        if op in (">=", ">"):  return l - r       # >= 0 cuando cumple
                        if op in ("<=", "<"):  return r - l       # >= 0 cuando cumple
                        if op == "==":         return -(abs(l - r))
                    except Exception:
                        return -1e6
                return fn
            ctype = "eq" if c.operator == "==" else "ineq"
            scipy_constraints.append({"type": ctype, "fun": make_fn(c, free_vars, statements)})

        polish_result = minimize(
            objective,
            x0=x_best,
            method="SLSQP",
            bounds=bounds,
            constraints=scipy_constraints,
            options={"maxiter": 5000, "ftol": 1e-12},
        )
        # Aceptar polish si reduce la violación real (no solo el objetivo)
        def total_pen(x):
            ctx = build_context(statements, dict(zip(free_vars, x)))
            return sum(_constraint_penalty(c, ctx) ** 2 for c in constraints)

        if total_pen(polish_result.x) <= total_pen(x_best):
            x_best = polish_result.x

    # ── Evaluar resultado ──────────────────────────────────────────────────────
    free_dict = dict(zip(free_vars, x_best))
    ctx_best  = build_context(statements, free_dict)

    total_penalty = sum(_constraint_penalty(c, ctx_best) ** 2 for c in constraints)
    feasible = total_penalty < feasibility_tol

    violated = []
    for c in constraints:
        if _constraint_penalty(c, ctx_best) > 1e-6:
            violated.append(c.raw)

    # ── Libertad (slack) de cada restricción ───────────────────────────────────
    libertad = []
    for c in constraints:
        try:
            l = eval_ast(c.left, ctx_best)
            r = eval_ast(c.right, ctx_best)
            op = c.operator
            if op == ">=":
                slack = l - r
            elif op == "<=":
                slack = r - l
            elif op == ">":
                slack = l - r
            elif op == "<":
                slack = r - l
            elif op == "==":
                slack = -(abs(l - r))  # siempre ≤ 0, no hay libertad en igualdades
            pen_val = _constraint_penalty(c, ctx_best)
            if pen_val < feasibility_tol:
                pen_val = 0.0
            if abs(slack) < feasibility_tol:
                slack = 0.0
            libertad.append({
                "label": c.label,
                "raw": c.raw,
                "slack": round(slack, 6),
                "penalty": round(pen_val, 8),
            })
        except Exception:
            libertad.append({
                "label": c.label,
                "raw": c.raw,
                "slack": None,
                "penalty": 1e6,
            })

    # ── Monte Carlo: probabilidad y efectividad ────────────────────────────────
    rng = np.random.default_rng(seed=0)
    samples = np.column_stack([
        rng.uniform(lo, hi, montecarlo_n) for lo, hi in bounds
    ])

    feasible_mask = np.zeros(montecarlo_n, dtype=bool)
    for i, row in enumerate(samples):
        free_i = dict(zip(free_vars, row))
        ctx_i  = build_context(statements, free_i)
        pen_i  = sum(_constraint_penalty(c, ctx_i) ** 2 for c in constraints)
        feasible_mask[i] = pen_i < feasibility_tol

    probability = float(feasible_mask.mean())

    # Efectividad: dentro de las factibles, las que están "cerca" del plan
    # (dentro del 20 % del rango de cada variable)
    if feasible_mask.sum() > 0 and feasible:
        radii = np.array([(hi - lo) * 0.20 for lo, hi in bounds])
        feasible_samples = samples[feasible_mask]
        near_plan = np.all(np.abs(feasible_samples - x_best) <= radii, axis=1)
        effectiveness = float(near_plan.mean())
    else:
        effectiveness = 0.0

    plan = {v: round(ctx_best.get(v, 0.0), 4) for v in list(free_dict) + list(extract_calculated_vars(statements))}

    return SolverResult(
        feasible=feasible,
        plan=plan,
        penalty=round(total_penalty, 8),
        strategy=strategy,
        probability=round(probability, 4),
        effectiveness=round(effectiveness, 4),
        montecarlo_samples=montecarlo_n,
        constraint_violations=violated,
        libertad=libertad,
    )

# =============================================================================
# SECCIÓN 7: DEMO
# =============================================================================

if __name__ == "__main__":
    import time
    # ── Caso 1: sistema lineal simple ─────────────────────────────────────────
    caso_1 = """
// Ramo con nota de presentación y examen
// Variables libres: NP, Ex
NF = NP * 0.6 + Ex * 0.4
NF >= 4.0
NP >= 1.0
NP in [1.0, 7.0]
Ex in [1.0, 7.0]
"""

    # ── Caso 2: con prom() ────────────────────────────────────────────────────
    caso_2 = """
// Controles y examen
PC = prom(C1, C2, C3)
NF = PC * 0.5 + Ex * 0.5
NF >= 4.0
PC >= 3.5
C1 in [1.0, 7.0]
C2 in [1.0, 7.0]
C3 in [1.0, 7.0]
Ex in [1.0, 7.0]
"""

    # ── Caso 3: con cada() — todos deben cumplir ──────────────────────────────
    caso_3 = """
// Todos los hitos deben ser >= 3.0 (cada = min)
H = cada(H1, H2, H3)
H >= 3.0
NF = H * 0.4 + Ex * 0.6
NF >= 4.0
H1 in [1.0, 7.0]
H2 in [1.0, 7.0]
H3 in [1.0, 7.0]
Ex in [1.0, 7.0]
"""

    # ── Caso 4: no-lineal real (producto de variables) ────────────────────────
    caso_4 = """
// Nota final tiene un componente multiplicativo (no-lineal!)
Bonus = A * B
NF = Base * 0.7 + Bonus * 0.3
NF >= 4.0
Bonus >= 10.0
Base in [1.0, 7.0]
A in [1.0, 5.0]
B in [1.0, 5.0]
"""

    caso_5 = """
// Dominio de variables libres (Escala USM: 0 a 100)
dominio C1, C2, C3 [0, 100]
dominio L1, L2, L3, L4, L5 [0, 100]
dominio g [1.0, 1.1]
// NC: Nota de Certámenes
NC = ( C3 * ((C1 + C2) / 2) ** 2 ) ** (1 / 3)
// NPL: Nota de Laboratorios considerando el peor borrado (n = 5)
NPL = ((L1 + L2 + L3 + L4 + L5) - min(L1, L2, L4)) / 4
// NL: Nota de Laboratorio con penalización escalón si NPL < 55
NL = NPL * escalon(NPL - 55)
// NF: Nota Final
Nota Final: NF = g * (0.75 * NC + 0.25 * NL)
// Restricciones de aprobación
Requisito de Aprobación: NF >= 55
"""

    casos = [
        ("Caso 1 - Lineal simple",    caso_1, "minimo_esfuerzo"),
        ("Caso 2 - prom()",            caso_2, "punto_medio"),
        ("Caso 3 - cada()",            caso_3, "maximo_seguridad"),
        ("Caso 4 - No lineal (A*B)",   caso_4, "minimo_varianza"),
        ("Caso 5 - CC min",        caso_5, "minimo_esfuerzo"),
        ("Caso 6 - CC max",        caso_5, "maximo_seguridad"),
        ("Caso 7 - CC mid",        caso_5, "punto_medio"),
        ("Caso 8 - CC var",        caso_5, "minimo_varianza"),
    ]

    for titulo, script, strat in casos:
        print(f"\n{'='*60}")
        print(f"  {titulo}  |  estrategia: {strat}")
        print('='*60)
        t0 = time.perf_counter()
        r = solve(script, strategy=strat, montecarlo_n=3000)
        elapsed = time.perf_counter() - t0
        print(f"  Tiempo         : {elapsed:.3f}s")
        print(f"  Factible       : {r.feasible}")
        print(f"  Penalización   : {r.penalty}")
        print(f"  Plan           : {json.dumps(r.plan, ensure_ascii=False)}")
        print(f"  Probabilidad   : {r.probability:.1%}")
        print(f"  Efectividad    : {r.effectiveness:.1%}")
        if r.constraint_violations:
            print(f"  Violaciones    : {r.constraint_violations}")
        if r.libertad:
            print(f"  Libertad (slack):")
            for lb in r.libertad:
                label = f"  [{lb['label']}] " if lb['label'] else "  "
                slack_str = f"{lb['slack']:.6f}" if lb['slack'] is not None else "N/A"
                print(f"{label}{lb['raw']:50s} slack={slack_str}  penalty={lb['penalty']}")
