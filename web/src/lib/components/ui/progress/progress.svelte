<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		ref = $bindable(null),
		class: className,
		value = 0,
		max = 100,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		value?: number;
		max?: number;
		ref?: HTMLDivElement | null;
	} = $props();

	const percentage = $derived(Math.min(Math.max((value / max) * 100, 0), 100));
</script>

<div
	bind:this={ref}
	class={cn('relative h-2 w-full overflow-hidden rounded-full bg-primary/20', className)}
	{...restProps}
>
	<div
		class="h-full w-full flex-1 bg-primary transition-all"
		style="transform: translateX(-{100 - percentage}%)"
	></div>
</div>