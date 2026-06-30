<script lang="ts">
	import { onMount } from 'svelte';
	import init, { solve, validate, JsSolverConfig } from '$lib/solver';

	let ready = $state(false);
	let running = $state(false);

	let script = $state(`X in [1, 10]
Y in [1, 10]
NF >= 2
NP >= 3
X + Y >= 5`);

	let strategy = $state('punto_medio');
	let defaultDomainLo = $state(0);
	let defaultDomainHi = $state(100);
	let penaltyWeight = $state(1_000_000);
	let montecarloN = $state(2000);
	let feasibilityTol = $state(0.0001);
	let popsize = $state(10);
	let maxIter = $state(1000);

	function buildCfg() {
		const cfg = new JsSolverConfig(strategy);
		if (defaultDomainLo) cfg.default_domain_lo = defaultDomainLo;
		if (defaultDomainHi) cfg.default_domain_hi = defaultDomainHi;
		if (penaltyWeight) cfg.penalty_weight = penaltyWeight;
		if (montecarloN) cfg.montecarlo_n = montecarloN;
		if (feasibilityTol) cfg.feasibility_tol = feasibilityTol;
		if (popsize) cfg.popsize = popsize;
		if (maxIter) cfg.max_iter = maxIter;
		return cfg;
	}

	type Result = {
		feasible: boolean;
		plan: Record<string, number>;
		penalty: number;
		strategy: string;
		probability: number;
		effectiveness: number;
		montecarlo_samples: number;
		constraint_violations: string[];
		libertad: { label?: string; raw: string; slack: number; penalty: number }[];
		elapsed_ms: number;
	};

	let result: Result | null = $state(null);
	let error: string | null = $state(null);
	let validationErrors: string[] = $state([]);

	let resultKey = $state(0);

	onMount(async () => {
		await init();
		ready = true;
		validar();
	});

	function validar() {
		if (!ready) return;
		try {
			const res = validate(script, buildCfg()) as any;
			validationErrors = res.errors ?? [];
		} catch {
			validationErrors = [];
		}
	}

	function ejecutar() {
		running = true;
		result = null;
		error = null;
		resultKey++;

	try {
			result = solve(script, buildCfg()) as Result;
		} catch (e) {
			error = String(e);
		}

		running = false;
	}

	const ejemplos: Record<string, string> = {
		'Básico': `NX in [1, 10]
NY in [1, 10]
NF >= 2
NP >= 3
NX + NY >= 5`,
		'Sumas': `X in [1, 10]
Y in [1, 10]
Z in [1, 10]
X + Y >= 6
Y + Z >= 4`,
		'1 > 0': `1 > 0`,
		'C1 >= 55': `C1 >= 55`,
		'Promedio': `a in [0, 10]
b in [0, 10]
prom = (a + b) / 2
prom >= 5`,
	};

	function cargarEjemplo(n: string) {
		script = ejemplos[n] ?? script;
	}

	function fmt(v: number): string {
		return Number.isInteger(v) ? v.toString() : v.toFixed(4);
	}

	$effect(() => {
		script;
		validar();
	});
</script>

<div class="min-h-dvh bg-neutral-50 text-neutral-900 antialiased">
	<header class="border-b border-neutral-200 bg-white">
		<div class="mx-auto flex max-w-6xl items-center gap-3 px-6 py-4">
			<span class="text-xl font-bold tracking-tight">RamoLibre Solver</span>
			<span class="rounded bg-neutral-100 px-2 py-0.5 text-xs font-medium text-neutral-500">WASM Test</span>
		</div>
	</header>

	<main class="mx-auto max-w-6xl px-6 py-6">
		<div class="grid gap-6 lg:grid-cols-2">
			<section class="space-y-5">
				<div class="rounded-lg border border-neutral-200 bg-white p-5 shadow-xs">
					<h2 class="mb-3 text-sm font-semibold uppercase tracking-wider text-neutral-500">Script</h2>
					<textarea
						bind:value={script}
						rows="8"
						disabled={running}
						class="w-full resize-y rounded-md border border-neutral-300 bg-neutral-50 px-3 py-2 font-mono text-sm leading-relaxed transition-colors placeholder:text-neutral-400 focus:border-neutral-400 focus:outline-hidden focus:ring-2 focus:ring-neutral-200 disabled:opacity-50"
						placeholder="Escribí el DSL aquí…"
					></textarea>

					<div class="mt-3 flex flex-wrap gap-2">
						{#each Object.keys(ejemplos) as name}
							<button
								onclick={() => cargarEjemplo(name)}
								disabled={running}
								class="cursor-pointer rounded-md border border-neutral-300 bg-white px-3 py-1 text-xs font-medium text-neutral-600 transition-colors hover:bg-neutral-100 hover:text-neutral-800 disabled:opacity-40"
							>{name}</button>
						{/each}
					</div>

					{#if validationErrors.length > 0}
						<div class="mt-3 rounded-md border border-red-200 bg-red-50 px-4 py-3">
							<p class="text-xs font-semibold uppercase tracking-wider text-red-600">Errores de validación</p>
							<ul class="mt-1 list-inside list-disc space-y-0.5 text-sm text-red-700">
								{#each validationErrors as e}
									<li>{e}</li>
								{/each}
							</ul>
						</div>
					{/if}

					<button
						onclick={ejecutar}
						disabled={!ready || running || validationErrors.length > 0}
						class="mt-4 w-full cursor-pointer rounded-md bg-neutral-900 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-neutral-800 disabled:cursor-not-allowed disabled:opacity-40"
					>
						{running ? 'Ejecutando…' : 'Ejecutar'}
					</button>
				</div>

				<div class="rounded-lg border border-neutral-200 bg-white p-5 shadow-xs">
					<h2 class="mb-3 text-sm font-semibold uppercase tracking-wider text-neutral-500">Configuración</h2>

					<div class="grid grid-cols-2 gap-x-4 gap-y-3">
						<label class="block">
							<span class="text-xs font-medium text-neutral-600">Estrategia</span>
							<select
								bind:value={strategy}
								disabled={running}
								class="mt-1 w-full rounded-md border border-neutral-300 bg-neutral-50 px-2 py-1.5 text-sm transition-colors focus:border-neutral-400 focus:outline-hidden focus:ring-2 focus:ring-neutral-200 disabled:opacity-50"
							>
								<option value="punto_medio">punto_medio</option>
								<option value="minimo_esfuerzo">minimo_esfuerzo</option>
								<option value="maximo_seguridad">maximo_seguridad</option>
								<option value="minimo_varianza">minimo_varianza</option>
							</select>
						</label>

						<label class="block">
							<span class="text-xs font-medium text-neutral-600">Muestras MC</span>
							<input
								type="number"
								bind:value={montecarloN}
								min="1"
								disabled={running}
								class="mt-1 w-full rounded-md border border-neutral-300 bg-neutral-50 px-2 py-1.5 text-sm transition-colors focus:border-neutral-400 focus:outline-hidden focus:ring-2 focus:ring-neutral-200 disabled:opacity-50"
							/>
						</label>

						<label class="block">
							<span class="text-xs font-medium text-neutral-600">Lo dominio</span>
							<input
								type="number"
								bind:value={defaultDomainLo}
								step="any"
								disabled={running}
								class="mt-1 w-full rounded-md border border-neutral-300 bg-neutral-50 px-2 py-1.5 text-sm transition-colors focus:border-neutral-400 focus:outline-hidden focus:ring-2 focus:ring-neutral-200 disabled:opacity-50"
							/>
						</label>

						<label class="block">
							<span class="text-xs font-medium text-neutral-600">Hi dominio</span>
							<input
								type="number"
								bind:value={defaultDomainHi}
								step="any"
								disabled={running}
								class="mt-1 w-full rounded-md border border-neutral-300 bg-neutral-50 px-2 py-1.5 text-sm transition-colors focus:border-neutral-400 focus:outline-hidden focus:ring-2 focus:ring-neutral-200 disabled:opacity-50"
							/>
						</label>

						<label class="block">
							<span class="text-xs font-medium text-neutral-600">Penalty weight</span>
							<input
								type="number"
								bind:value={penaltyWeight}
								min="0"
								step="any"
								disabled={running}
								class="mt-1 w-full rounded-md border border-neutral-300 bg-neutral-50 px-2 py-1.5 text-sm transition-colors focus:border-neutral-400 focus:outline-hidden focus:ring-2 focus:ring-neutral-200 disabled:opacity-50"
							/>
						</label>

						<label class="block">
							<span class="text-xs font-medium text-neutral-600">Feasibility tol</span>
							<input
								type="number"
								bind:value={feasibilityTol}
								min="0"
								step="any"
								disabled={running}
								class="mt-1 w-full rounded-md border border-neutral-300 bg-neutral-50 px-2 py-1.5 text-sm transition-colors focus:border-neutral-400 focus:outline-hidden focus:ring-2 focus:ring-neutral-200 disabled:opacity-50"
							/>
						</label>

						<label class="block">
							<span class="text-xs font-medium text-neutral-600">Población (popsize)</span>
							<input
								type="number"
								bind:value={popsize}
								min="1"
								disabled={running}
								class="mt-1 w-full rounded-md border border-neutral-300 bg-neutral-50 px-2 py-1.5 text-sm transition-colors focus:border-neutral-400 focus:outline-hidden focus:ring-2 focus:ring-neutral-200 disabled:opacity-50"
							/>
						</label>

						<label class="block">
							<span class="text-xs font-medium text-neutral-600">Máx. iteraciones</span>
							<input
								type="number"
								bind:value={maxIter}
								min="1"
								disabled={running}
								class="mt-1 w-full rounded-md border border-neutral-300 bg-neutral-50 px-2 py-1.5 text-sm transition-colors focus:border-neutral-400 focus:outline-hidden focus:ring-2 focus:ring-neutral-200 disabled:opacity-50"
							/>
						</label>
					</div>
				</div>
			</section>

			<section class="space-y-5">
				<div class="rounded-lg border border-neutral-200 bg-white p-5 shadow-xs">
					<h2 class="mb-3 text-sm font-semibold uppercase tracking-wider text-neutral-500">Resultado</h2>

					{#if !ready}
						<p class="py-8 text-center text-sm text-neutral-400">Cargando WASM…</p>

					{:else if error}
						<div class="rounded-md border border-red-200 bg-red-50 px-4 py-3">
							<p class="text-xs font-semibold uppercase tracking-wider text-red-600">Error</p>
							<pre class="mt-1 overflow-x-auto whitespace-pre-wrap font-mono text-sm text-red-700">{error}</pre>
						</div>

					{:else if running}
						<p class="py-8 text-center text-sm text-neutral-400">Ejecutando solver…</p>

					{:else if result}
						<div class="space-y-5">
							<div class="flex items-center gap-3">
								<span
									class="inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-semibold {result.feasible
										? 'bg-green-50 text-green-700 ring-1 ring-green-200'
										: 'bg-red-50 text-red-700 ring-1 ring-red-200'}"
								>
									<span class="h-1.5 w-1.5 rounded-full {result.feasible ? 'bg-green-500' : 'bg-red-500'}"></span>
									{result.feasible ? 'Factible' : 'No factible'}
								</span>
								<span class="text-xs text-neutral-400">{result.strategy}</span>
							</div>

							<div class="grid grid-cols-3 gap-3">
								<div class="rounded-md bg-neutral-50 px-3 py-2">
									<p class="text-[10px] font-semibold uppercase tracking-wider text-neutral-500">Penalización</p>
									<p class="mt-0.5 font-mono text-sm font-medium">{result.penalty}</p>
								</div>
								<div class="rounded-md bg-neutral-50 px-3 py-2">
									<p class="text-[10px] font-semibold uppercase tracking-wider text-neutral-500">Tiempo</p>
									<p class="mt-0.5 font-mono text-sm font-medium">{result.elapsed_ms} ms</p>
								</div>
								<div class="rounded-md bg-neutral-50 px-3 py-2">
									<p class="text-[10px] font-semibold uppercase tracking-wider text-neutral-500">Probabilidad</p>
									<p class="mt-0.5 font-mono text-sm font-medium">{(result.probability * 100).toFixed(1)}%</p>
								</div>
								<div class="rounded-md bg-neutral-50 px-3 py-2">
									<p class="text-[10px] font-semibold uppercase tracking-wider text-neutral-500">Efectividad</p>
									<p class="mt-0.5 font-mono text-sm font-medium">{(result.effectiveness * 100).toFixed(1)}%</p>
								</div>
								<div class="rounded-md bg-neutral-50 px-3 py-2">
									<p class="text-[10px] font-semibold uppercase tracking-wider text-neutral-500">Muestras MC</p>
									<p class="mt-0.5 font-mono text-sm font-medium">{result.montecarlo_samples}</p>
								</div>
							</div>

							<div>
								<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-neutral-500">Plan</h3>
								{#if Object.keys(result.plan).length > 0}
									<table class="w-full text-sm">
										<thead>
											<tr class="border-b border-neutral-200 text-left text-xs font-medium text-neutral-500">
												<th class="pb-1.5 pr-4">Variable</th>
												<th class="pb-1.5">Valor</th>
											</tr>
										</thead>
										<tbody>
											{#each Object.entries(result.plan).sort((a, b) => a[0].localeCompare(b[0])) as [k, v]}
												<tr class="border-b border-neutral-100">
													<td class="py-1.5 pr-4 font-mono font-medium text-neutral-900">{k}</td>
													<td class="py-1.5 font-mono text-neutral-700">{fmt(v)}</td>
												</tr>
											{/each}
										</tbody>
									</table>
								{:else}
									<p class="text-sm text-neutral-400 italic">(sin variables libres)</p>
								{/if}
							</div>

							{#if result.libertad && result.libertad.length > 0}
								<div>
									<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-neutral-500">Libertad (slack)</h3>
									<div class="overflow-x-auto rounded-md border border-neutral-200">
										<table class="w-full text-sm">
											<thead>
												<tr class="bg-neutral-50 text-left text-xs font-medium text-neutral-500">
													<th class="px-3 py-2">Restricción</th>
													<th class="px-3 py-2">Slack</th>
													<th class="px-3 py-2">Penalty</th>
												</tr>
											</thead>
											<tbody>
												{#each result.libertad as e}
													<tr class="border-t border-neutral-100">
														<td class="max-w-64 truncate px-3 py-1.5 font-mono text-xs text-neutral-700" title={e.raw}>{e.label ? `[${e.label}] ` : ''}{e.raw}</td>
														<td class="px-3 py-1.5 font-mono text-xs {e.slack < 0 ? 'text-red-600' : 'text-green-700'}">{e.slack.toFixed(6)}</td>
														<td class="px-3 py-1.5 font-mono text-xs text-neutral-500">{e.penalty.toExponential(2)}</td>
													</tr>
												{/each}
											</tbody>
										</table>
									</div>
								</div>
							{/if}

							{#if result.constraint_violations && result.constraint_violations.length > 0}
								<div>
									<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-neutral-500">Violaciones</h3>
									<ul class="space-y-1">
										{#each result.constraint_violations as v}
											<li class="rounded-md border border-red-200 bg-red-50 px-3 py-1.5 font-mono text-xs text-red-700">{v}</li>
										{/each}
									</ul>
								</div>
							{/if}
						</div>

					{:else}
						<p class="py-8 text-center text-sm text-neutral-400">
							Presioná <span class="font-medium text-neutral-600">Ejecutar</span> para ver el resultado
						</p>
					{/if}
				</div>
			</section>
		</div>
	</main>
</div>
