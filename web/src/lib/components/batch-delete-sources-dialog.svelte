<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Button } from '$lib/components/ui/button';
	import api from '$lib/api';
	import {
		batchDeleteSources,
		sourceSelectionKey,
		type BatchDeleteSource,
		type BatchDeleteResult
	} from '$lib/batch-delete-sources.js';

	export let isOpen = false;
	export let sources: BatchDeleteSource[] = [];
	const dispatch = createEventDispatcher<{ complete: BatchDeleteResult[] }>();
	let deleteLocalFiles = false;
	let deleting = false;
	let finished = false;
	let results: BatchDeleteResult[] = [];

	$: if (isOpen) {
		deleteLocalFiles = false;
		finished = false;
		results = [];
	}
	$: accepted = results.filter((result) => result.success).length;
	$: failed = results.length - accepted;

	async function confirmDelete() {
		if (deleting || finished || sources.length === 0) return;
		deleting = true;
		try {
			const completed = await batchDeleteSources(
				sources,
				api.deleteVideoSource,
				deleteLocalFiles,
				(result) => (results = [...results, result])
			);
			finished = true;
			dispatch('complete', completed);
		} finally {
			deleting = false;
		}
	}
</script>

<AlertDialog.Root bind:open={isOpen}>
	<AlertDialog.Content
		class="max-h-[90vh] overflow-y-auto sm:max-w-xl"
		onEscapeKeydown={(event) => {
			if (deleting) event.preventDefault();
		}}
	>
		<AlertDialog.Header>
			<AlertDialog.Title>批量删除 {sources.length} 个视频源</AlertDialog.Title>
			<AlertDialog.Description>
				删除所选视频源及仅属于这些源的视频记录和下载历史。默认保留本地文件。
				正在扫描时，删除请求会排队处理。
			</AlertDialog.Description>
		</AlertDialog.Header>
		<ul class="max-h-64 space-y-2 overflow-y-auto rounded-md border p-3 text-sm">
			{#each sources as source (sourceSelectionKey(source.type, source.id))}
				{@const result = results.find(
					(item) => item.source.type === source.type && item.source.id === source.id
				)}
				<li class="space-y-1 break-all">
					<p class="font-medium">{source.typeLabel} · {source.name}</p>
					<p class="text-muted-foreground text-xs">
						{#if source.externalId}ID: {source.externalId} ·
						{/if}{source.path}
					</p>
					{#if result}
						<p class={result.success ? 'text-muted-foreground' : 'text-destructive'}>
							{result.success ? '请求已接受' : '失败'}：{result.message}
						</p>
					{/if}
				</li>
			{/each}
		</ul>
		{#if !finished}
			<label class="flex items-start gap-2 text-sm">
				<input
					type="checkbox"
					bind:checked={deleteLocalFiles}
					disabled={deleting}
					class="mt-1 h-4 w-4"
				/>
				<span>同时永久删除这些视频源的本地下载文件</span>
			</label>
		{/if}
		<p role="status" class="text-muted-foreground text-sm">
			{#if deleting}
				已处理 {results.length} / {sources.length} 个请求
			{:else if finished}
				已接受 {accepted} 个删除请求，失败 {failed} 个。
				{#if failed > 0}失败的源仍被选中，可关闭此窗口后重试。{/if}
			{:else}
				视频源及相关数据库记录删除后无法恢复。
			{/if}
		</p>
		<AlertDialog.Footer>
			<Button variant="outline" disabled={deleting} onclick={() => (isOpen = false)}>
				{finished ? '关闭' : '取消'}
			</Button>
			{#if !finished}
				<Button
					variant="destructive"
					disabled={deleting || sources.length === 0}
					onclick={confirmDelete}
				>
					{deleting ? '正在提交…' : `删除 ${sources.length} 个视频源`}
				</Button>
			{/if}
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
