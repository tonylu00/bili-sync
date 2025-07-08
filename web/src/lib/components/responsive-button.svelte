<script lang="ts">
	import { Button } from '$lib/components/ui/button';

	export let icon: any;
	export let text: string;
	export let hideTextOnMobile: boolean = true;
	export let size: any = 'default';
	export let variant: any = 'default';
	export let onclick: (() => void) | undefined = undefined;
	export let title: string = '';
	
	let className: string = '';
	export { className as class };
</script>

<Button
	{size}
	{variant}
	{onclick}
	{title}
	class="flex items-center gap-2 {hideTextOnMobile ? 'sm:gap-2' : 'gap-2'} {className}"
>
	<svelte:component this={icon} class="h-4 w-4" />
	{#if hideTextOnMobile}
		<span class="hidden sm:inline">{text}</span>
	{:else}
		<span>{text}</span>
	{/if}
</Button>