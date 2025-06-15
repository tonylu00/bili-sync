<script lang="ts">
	import { formatTimestamp, getRelativeTime, getCurrentTimezone } from '$lib/utils/timezone';
	import { onMount } from 'svelte';

	export let timestamp: string | number | Date;
	export let format: 'datetime' | 'date' | 'time' | 'relative' = 'datetime';
	export let timezone: string = '';
	export let showTooltip: boolean = true;

	let currentTimezone = '';
	let formattedTime = '';
	let relativeTime = '';

	onMount(() => {
		currentTimezone = timezone || getCurrentTimezone();
		updateTime();
	});

	function updateTime() {
		if (format === 'relative') {
			formattedTime = getRelativeTime(timestamp, currentTimezone);
			relativeTime = formatTimestamp(timestamp, currentTimezone, 'datetime');
		} else {
			formattedTime = formatTimestamp(timestamp, currentTimezone, format);
			relativeTime = getRelativeTime(timestamp, currentTimezone);
		}
	}

	// 响应时区变化
	$: if (currentTimezone !== (timezone || getCurrentTimezone())) {
		currentTimezone = timezone || getCurrentTimezone();
		updateTime();
	}

	// 响应时间戳变化
	$: if (timestamp) {
		updateTime();
	}
</script>

{#if showTooltip && format === 'relative'}
	<span title={relativeTime} class="cursor-help">
		{formattedTime}
	</span>
{:else if showTooltip && format !== 'relative'}
	<span title={relativeTime} class="cursor-help">
		{formattedTime}
	</span>
{:else}
	<span>{formattedTime}</span>
{/if} 