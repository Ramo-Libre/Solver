<script lang="ts">
	import { onMount } from 'svelte';
	import init, { solve, validate } from '$lib/solver/solver.js';

	let ready = $state(false);
	let script = $state(`NF >= 2;
NP >= 3;
NX + NY >= 5;`);
	let result: any = $state(null);
	let error = $state('');
	let validationErrors: string[] = $state([]);
	let running = $state(false);

	const cfg = {
		strategy: 'minimo_esfuerzo',
		montecarlo: 3000,
		feasibilityTol: 1e-6,
	};

	onMount(async () => {
		try {
			await init();
			ready = true;
			validar();
		} catch (e) {
			error = String(e);
		}
	});

	function validar() {
		if (!ready) return;
		try {
			const res = validate(script, cfg) as any;
			validationErrors = res.errors ?? [];
		} catch {
			validationErrors = [];
		}
	}

	function ejecutar() {
		running = true;
		result = null;
		error = '';

		try {
			result = solve(script, cfg);
		} catch (e) {
			error = String(e);
		}

		running = false;
	}

	function cargarEjemplo(n: number) {
		const ejemplos: Record<number, string> = {
			1: `NX in [1, 10];
NY in [1, 10];
NF >= 2;
NP >= 3;
NX + NY >= 5;`,
			2: `X in [1, 10];
Y in [1, 10];
Z in [1, 10];
X + Y >= 6;
Y + Z >= 4;`,
			3: `1 > 0`,
			4: `C1 >= 55`,
		};
		script = ejemplos[n] ?? script;
	}
</script>

<h1>RamoLibre Solver — WASM Test</h1>

<div class="layout">
	<div class="panel">
		<h2>Script</h2>
		<textarea
			bind:value={script}
			rows="10"
			disabled={running}
			oninput={validar}
		></textarea>
		<div class="toolbar">
			<button onclick={() => cargarEjemplo(1)} disabled={running}>Ej. 1</button>
			<button onclick={() => cargarEjemplo(2)} disabled={running}>Ej. 2</button>
			<button onclick={() => cargarEjemplo(3)} disabled={running}>1 > 0</button>
			<button onclick={() => cargarEjemplo(4)} disabled={running}>C1 >= 55</button>
			<button
				class="run"
				onclick={ejecutar}
				disabled={!ready || running || validationErrors.length > 0}
			>
				{running ? 'Ejecutando…' : 'Ejecutar'}
			</button>
		</div>

		{#if validationErrors.length > 0}
			<div class="validation">
				<h3>Errores de validación</h3>
				<ul>
					{#each validationErrors as e}
						<li>{e}</li>
					{/each}
				</ul>
			</div>
		{/if}

		{#if !ready}
			<p class="msg">Cargando WASM…</p>
		{/if}
	</div>

	<div class="panel">
		<h2>Resultado</h2>

		{#if error}
			<pre class="error">{error}</pre>
		{:else if running}
			<p class="msg">Ejecutando solver…</p>
		{:else if result}
			<table>
				<tbody>
					<tr>
						<td class="label">Factible</td>
						<td class={result.feasible ? 'ok' : 'fail'}>{result.feasible}</td>
					</tr>
					<tr><td class="label">Penalización</td><td>{result.penalty}</td></tr>
					<tr><td class="label">Tiempo</td><td>{result.elapsed_ms} ms</td></tr>
					<tr><td class="label">Probabilidad</td><td>{result.probability}</td></tr>
					<tr><td class="label">Efectividad</td><td>{result.effectiveness}</td></tr>
					<tr><td class="label">Muestras MC</td><td>{result.montecarlo_samples}</td></tr>
				</tbody>
			</table>

			<h3>Plan</h3>
			<table>
				<tbody>
					{#each Object.entries(result.plan) as [k, v]}
						<tr><td class="label">{k}</td><td>{v}</td></tr>
					{/each}
				</tbody>
			</table>

			{#if result.libertad && result.libertad.length > 0}
				<h3>Libertad (slack)</h3>
				<table>
					<thead>
						<tr><th>Restricción</th><th>Slack</th><th>Penalty</th></tr>
					</thead>
					<tbody>
						{#each result.libertad as e}
							<tr>
								<td class="mono">{e.raw}</td>
								<td class={e.slack < 0 ? 'fail' : 'ok'}>{e.slack.toFixed(6)}</td>
								<td>{e.penalty.toExponential(2)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}

			{#if result.constraint_violations && result.constraint_violations.length > 0}
				<h3>Violaciones / Errores</h3>
				<ul>
					{#each result.constraint_violations as v}
						<li class="fail">{v}</li>
					{/each}
				</ul>
			{/if}
		{:else}
			<p class="msg">Presioná <strong>Ejecutar</strong></p>
		{/if}
	</div>
</div>

<style>
	h1 { margin: 0 0 1rem; font-size: 1.4rem; }

	.layout { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; align-items: start; }

	.panel {
		border: 1px solid #ccc; border-radius: 6px; padding: 1rem; background: #fafafa;
	}

	.panel h2 { margin: 0 0 .6rem; font-size: 1.1rem; }
	.panel h3 { margin: 1rem 0 .4rem; font-size: .95rem; }

	textarea {
		width: 100%; box-sizing: border-box;
		font-family: 'Courier New', Courier, monospace; font-size: .85rem;
		border: 1px solid #bbb; border-radius: 4px; padding: 6px; resize: vertical;
	}

	textarea:invalid { border-color: #d99; }

	.toolbar { display: flex; gap: 6px; margin-top: 8px; flex-wrap: wrap; }

	button {
		padding: 5px 12px; font-size: .85rem; border: 1px solid #888;
		border-radius: 4px; background: #fff; cursor: pointer;
	}

	button:hover:not(:disabled) { background: #eef; }
	button:disabled { opacity: .5; cursor: default; }
	button.run { background: #282; color: #fff; border-color: #282; }
	button.run:hover:not(:disabled) { background: #1a6; }

	.validation {
		margin-top: 8px; padding: 8px; background: #fee;
		border: 1px solid #d99; border-radius: 4px; font-size: .8rem;
	}

	.validation h3 { margin: 0 0 4px; font-size: .85rem; color: #b22; }
	.validation ul { margin: 0; padding-left: 1.2rem; }
	.validation li { color: #b22; margin-bottom: 2px; }

	table { width: 100%; border-collapse: collapse; font-size: .85rem; }
	td, th { padding: 3px 6px; text-align: left; border-bottom: 1px solid #ddd; }
	th { font-weight: 600; background: #eee; }
	.label { font-weight: 600; color: #555; width: 1px; white-space: nowrap; padding-right: 1rem; }
	.mono { font-family: 'Courier New', Courier, monospace; font-size: .8rem; word-break: break-all; }
	.ok { color: #282; }
	.fail { color: #b22; }

	.msg { color: #888; font-style: italic; }
	.error {
		color: #b22; background: #fee; border: 1px solid #d99;
		border-radius: 4px; padding: 8px; white-space: pre-wrap; font-size: .8rem;
	}
</style>
