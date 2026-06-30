# Solver

Librería, bindings y CLI para resolver sistemas de ecuaciones no lineales como problemas de optimización con 4 estrategias. Usa un DSL compartido por varios repos de Ramo Libre.

## Índice

- [DSL](#dsl)
- [Bindings](#bindings)
  - [TypeScript (napi-rs)](#typescript-napi-rs)
  - [WebAssembly (wasm-bindgen)](#webassembly-wasm-bindgen)
- [CLI](#cli)

---

## DSL

El DSL se escribe línea por línea. Cada línea puede ser:

### Comentarios

```
// esto es un comentario
```

### Asignación

```
NF = NP * 0.6 + Ex * 0.4
promedio = (a + b) / 2
```

Una variable definida en una asignación se considera **calculada** (no es variable libre de decisión). No puede tener el mismo nombre que una variable libre.

### Restricciones

```
NF >= 4.0
NP + Ex <= 10.0
a > 0
b < 100
a + b == 15
promedio <= 7.0
```

Operadores relacionales: `>=`, `<=`, `>`, `<`, `==`.

### Declaración de dominio (variable libre)

Forma 1 — keyword `dominio`:

```
dominio NP, Ex [1.0, 7.0]
dominio X [0, 100]
```

Forma 2 — notación `in`:

```
NP in [1.0, 7.0]
Ex in [1.0, 7.0]
```

Las variables con dominio son **variables libres de decisión**. El solver encuentra sus valores óptimos dentro del rango.

### Labels

```
critica: NF >= 4.0
suave: NP <= 7.0
```

Un label es un identificador antes de `:`. Se muestra en el slack output. No puede contener operadores (`=`, `<`, `>`, `+`, `-`, `*`, `/`, `(`, `)`, `[`, `]`).

### Expresiones aritméticas

Operadores: `+`, `-`, `*`, `/`, `**` (potencia).

```
a + b * c
(x - 1) ** 2 + (y - 2) ** 2
```

Funciones integradas:

```
prom(x...)
cada(x...)
escalon(x)
min(a, b, ...)
max(a, b, ...)
```

---

## Bindings

### TypeScript (napi-rs)

El binding napi-rs se compila a un `.node` file y se carga con `require()`.

```ts
import { solve, JsSolverConfig, JsSolverResult } from 'solver'
```

#### `JsSolverConfig` (input)

```ts
interface JsSolverConfig {
  strategy: 'punto_medio' | 'minimo_esfuerzo' | 'maximo_seguridad' | 'minimo_varianza'
  default_domain_lo?: number   // default 0.0
  default_domain_hi?: number   // default 100.0
  penalty_weight?: number      // default 1e6
  montecarlo_n?: number        // default 2000
  feasibility_tol?: number     // default 1e-4
  popsize?: number             // default 10
  max_iter?: number            // default 1000
}
```

#### `JsSolverResult` (output)

```ts
interface JsSolverResult {
  feasible: boolean
  plan: Record<string, number>        // variable → valor óptimo
  penalty: number
  strategy: string
  probability: number                 // 0.0 – 1.0
  effectiveness: number               // 0.0 – 1.0
  montecarlo_samples: number
  constraint_violations: string[]     // descripciones de violaciones
  libertad: JsLibertadEntry[]
  elapsed_ms: number
}

interface JsLibertadEntry {
  label?: string
  raw: string        // línea original de la constraint
  slack: number      // distancia al límite (negativo si viola)
  penalty: number    // penalización cuadrática
}
```

### WebAssembly (wasm-bindgen)

El WASM se compila con `wasm-pack` y expone las mismas funciones con serialización JSON nativa (`serde-wasm-bindgen`).

#### `solve(script: string, config: object): object`

```ts
import init, { solve, validate } from './solver'

await init()

const result = solve('x >= 5', {
  strategy: 'punto_medio',
  default_domain_lo: 0,
  default_domain_hi: 100,
})
// result.feasible: boolean
// result.plan: Record<string, number>
// result.constraint_violations: string[]
// result.libertad: { label?, raw, slack, penalty }[]
```

#### `validate(script: string, config: object): { valid: boolean, errors: string[] }`

Valida el script sin ejecutar el solver. Útil para feedback en tiempo real.

```ts
const { valid, errors } = validate('x >= 5', { strategy: 'punto_medio' })
// valid: true
// errors: []
```

---

## CLI

### Compilar

```
cargo build --release
alias ramo_solver='./target/release/solver'
```

### Uso

```
ramo_solver [opciones] < archivo.dsl
echo 'x >= 5' | ramo_solver
```

### Opciones

| Flag | Descripción | Default |
|---|---|---|
| `--strategy <nombre>` | `punto_medio`, `minimo_esfuerzo`, `maximo_seguridad`, `minimo_varianza` | `punto_medio` |
| `--montecarlo <n>` | Muestras Monte Carlo | `2000` |
| `--popsize <n>` | Población DE por dimensión | `10` |
| `--penalty <f>` | Peso de penalización | `1000000` |
| `--domain <lo,hi>` | Dominio por defecto si no se declara en el DSL | `0.0,100.0` |
| `--feasibility-tol <f>` | Tolerancia de factibilidad | `0.0001` |
| `--pretty` | Output legible para humanos | off (JSON) |
| `--json` | Output JSON (default) | on |
| `--help` | Muestra ayuda | |

### Ejemplos

```
echo 'x >= 5' | ramo_solver
echo 'x >= 5' | ramo_solver --pretty
echo 'x >= 5\nx in [0, 10]' | ramo_solver --strategy minimo_esfuerzo --montecarlo 5000
ramo_solver --domain 1.0,10.0 < problema.dsl
```
