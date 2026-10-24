<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { DedupOperation, DedupPlan } from '$lib/types';
	export let oncomplete: () => void = () => {};
	let plan: DedupPlan | null = null;
	let history: DedupOperation[] = [];
	let state = '';
	let busy = false;
	let error = '';
	let limit = 30;
	$: eligible = plan?.groups.filter((g) => !g.conflict) ?? [];
	$: total = eligible.reduce((sum, g) => sum + g.reclaimable_bytes, 0);
	function bytes(n: number) {
		return `${(n / 1024 ** 3).toFixed(2)} GiB`;
	}
	function message(e: unknown) {
		return e && typeof e === 'object' && 'message' in e
			? String(e.message)
			: '操作失败，请查看日志后重试';
	}
	function select(op: DedupOperation) {
		plan = op.plan;
		state = op.state;
		error = op.error ?? '';
		limit = 30;
	}
	async function refresh() {
		try {
			history = (await api.getDedupHistory()).data;
			const current = history.find((op) => op.id === plan?.id) ?? history[0];
			if (current) select(current);
		} catch (e) {
			error = message(e);
		}
	}
	async function preview() {
		busy = true;
		error = '';
		try {
			plan = (await api.previewDedup()).data;
			state = 'preview';
			limit = 30;
		} catch (e) {
			error = message(e);
		} finally {
			busy = false;
		}
	}
	async function execute() {
		if (!plan || busy) return;
		busy = true;
		error = '';
		state = 'running';
		try {
			await api.executeDedup(plan.id);
			state = 'complete';
			oncomplete();
		} catch (e) {
			error = message(e);
			state = 'interrupted';
		} finally {
			busy = false;
			await refresh();
		}
	}
	onMount(refresh);
</script>

<section class="space-y-4 rounded-lg border p-5" aria-label="合并重复媒体">
	<h2 class="text-lg font-semibold">合并重复媒体</h2>
	<p class="text-muted-foreground text-sm">
		同一 BV 的每个分 P
		只保存一份媒体。依次保留主投稿者文件夹、按顺序的联合创作者文件夹、最先添加的合集或收藏夹中的文件。
		其他来源通过应用内播放访问同一文件；来源目录仍保留元数据，不再各放一份视频。
	</p>
	<div class="flex flex-wrap gap-3">
		<button
			class="rounded-md border px-4 py-2 disabled:opacity-50"
			disabled={busy || state === 'running'}
			onclick={preview}
		>
			{busy && state !== 'running' ? '正在检查媒体与校验副本…' : '扫描并预览'}
		</button>
		<button
			class="rounded-md border px-4 py-2 disabled:opacity-50"
			disabled={busy}
			onclick={refresh}>刷新进度</button
		>
		{#if history.length > 1}
			<select
				class="bg-background rounded-md border px-3 py-2"
				aria-label="迁移记录"
				disabled={busy}
				onchange={(e) => {
					const op = history.find((o) => o.id === e.currentTarget.value);
					if (op) select(op);
				}}
			>
				<option value="">查看历史记录</option>
				{#each history as op}<option value={op.id}
						>{op.id.slice(0, 8)} · {op.state === 'complete'
							? '已完成'
							: op.state === 'interrupted'
								? '已中断'
								: op.state === 'running'
									? '正在合并'
									: '预览'}</option
					>{/each}
			</select>
		{/if}
	</div>
	{#if busy || state === 'running'}<p role="status" class="text-sm">
			正在处理；期间下载、删除和路径调整会等待。可以稍后刷新查看结果。
		</p>{/if}
	{#if error}<p role="alert" class="text-destructive text-sm break-words">{error}</p>{/if}
	{#if plan}
		<p class="text-sm">
			{eligible.length} 组可以合并，预计最多释放 {bytes(total)}；{plan.groups.length -
				eligible.length} 组冲突保留。
			{#if plan.unresolved_pages}另有 {plan.unresolved_pages} 个分 P 尚未确认身份，保持原样。{/if}
		</p>
		<p class="text-muted-foreground text-xs">
			只合并下载完成、媒体检查通过且内容校验完全相同的文件。不同质量、不可读或未完成的副本保留；未知硬链接可能使实际释放空间少于估算。
		</p>
		{#each plan.groups.slice(0, limit) as group (`${group.bvid}:${group.cid}`)}
			<details class="rounded border p-3">
				<summary class="cursor-pointer text-sm"
					>{group.keeper.name} · {group.bvid} · 分 P CID {group.cid} · {group.conflict
						? '需要检查'
						: bytes(group.reclaimable_bytes)}</summary
				>
				<p class="mt-3 text-sm break-all">保留：{group.keeper.path}（{group.reason}）</p>
				{#if group.conflict}<p class="mt-2 text-sm text-amber-600">{group.conflict}</p>{/if}
				<ul class="mt-2 space-y-1 text-xs">
					{#each [...new Set(group.copies.map((c) => c.path))] as path}<li class="break-all">
							{path}
						</li>{/each}
				</ul>
			</details>
		{/each}
		{#if plan.groups.length > limit}<button
				class="text-primary underline"
				onclick={() => (limit += 30)}>显示更多</button
			>{/if}
		{#if state === 'complete'}
			<p role="status" class="text-sm text-green-600">
				此次合并已完成。冲突副本保持原样，磁盘统计已刷新。
			</p>
		{:else if eligible.length > 0}
			<p class="text-sm">执行前会备份数据库。核对保留路径后，合并上述可处理组；冲突不会被删除。</p>
			<button
				class="bg-primary text-primary-foreground rounded-md px-4 py-2 disabled:opacity-50"
				disabled={busy || state === 'running'}
				onclick={execute}
			>
				{state === 'interrupted' ? '重新校验并继续合并' : '确认保留路径并合并'}
			</button>
		{/if}
	{/if}
</section>
