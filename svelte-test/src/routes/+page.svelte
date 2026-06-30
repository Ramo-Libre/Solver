<script lang="ts">
	import { onMount } from 'svelte';
	import init, { solve } from '$lib/solver/solver.js';

	let status = $state('Cargando...');
	let result: any = $state(null);
	let error = $state('');

	onMount(async () => {
		try {
			status = 'Inicializando WASM...';
			const wasm = await init();
			status = 'WASM iniciado OK';

			status = 'Ejecutando solve minimo...';
			const script = `variables X; minimizar X; X >= 1;`;

			result = solve(script, {
				strategy: 'minimo_esfuerzo',
				montecarlo_n: 100,
				feasibility_tol: 1e-6
			});

			status = 'Listo';
		} catch (e) {
			error = String(e) + '\n' + (e instanceof Error ? e.stack : '');
			status = 'Error';
		}
	});
</script>

<h1>Prueba Solver WASM</h1>

{#if status === 'Error'}
	<pre style="color: red; white-space: pre-wrap">{error}</pre>
{:else if status === 'Listo' && result}
	<p><strong>Factible:</strong> {result.feasible}</p>
	<p><strong>Penalización:</strong> {result.penalty}</p>
	<p><strong>Tiempo:</strong> {result.elapsed_ms}ms</p>
	<p><strong>Plan:</strong></p>
	<ul>
		{#each Object.entries(result.plan) as [k, v]}
			<li>{k} = {v}</li>
		{/each}
	</ul>
	{#if result.libertad}
		<p><strong>Libertad (slack):</strong></p>
		<ul>
			{#each result.libertad as e}
				<li>{e.raw}: slack={e.slack.toFixed(6)} penalty={e.penalty.toExponential(2)}</li>
			{/each}
		</ul>
	{/if}
	<p><strong>Violaciones:</strong> {JSON.stringify(result.constraint_violations)}</p>
{:else}
	<p>{status}</p>
{/if}
