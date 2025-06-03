<script lang="ts">
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Button } from '$lib/components/ui/button';
	import { createEventDispatcher } from 'svelte';

	export let isOpen = false;
	export let sourceName = '';
	export let sourceType = '';

	const dispatch = createEventDispatcher<{
		confirm: { deleteLocalFiles: boolean };
		cancel: void;
	}>();

	let deleteLocalFiles = false;
	let isDeleting = false;
	let confirmText = '';

	// 重置状态
	function resetState() {
		deleteLocalFiles = false;
		isDeleting = false;
		confirmText = '';
	}

	// 当对话框打开时重置状态
	$: if (isOpen) {
		resetState();
	}

	// 获取视频源类型的中文名称
	function getSourceTypeLabel(type: string): string {
		const typeMap: Record<string, string> = {
			collection: '合集',
			favorite: '收藏夹',
			submission: 'UP主投稿',
			watch_later: '稍后观看',
			bangumi: '番剧'
		};
		return typeMap[type] || type;
	}

	// 处理确认删除
	async function handleConfirm() {
		if (confirmText !== sourceName) {
			return;
		}

		isDeleting = true;
		try {
			dispatch('confirm', { deleteLocalFiles });
			isOpen = false;
		} catch (error) {
			console.error('删除失败:', error);
		} finally {
			isDeleting = false;
		}
	}

	// 处理取消
	function handleCancel() {
		if (isDeleting) return;
		dispatch('cancel');
		isOpen = false;
	}

	// 检查是否可以确认删除
	$: canConfirm = confirmText === sourceName && !isDeleting;
</script>

<AlertDialog.Root bind:open={isOpen}>
	<AlertDialog.Content class="max-w-md">
		<AlertDialog.Header>
			<AlertDialog.Title class="flex items-center gap-2 text-destructive">
				<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.5 0L4.268 19.5c-.77.833.192 2.5 1.732 2.5z" />
				</svg>
				删除视频源
			</AlertDialog.Title>
			<AlertDialog.Description class="space-y-3">
				<div class="rounded-lg bg-red-50 p-3 border border-red-200">
					<p class="text-sm text-red-800 font-medium">⚠️ 危险操作警告</p>
					<p class="text-xs text-red-700 mt-1">
						此操作将永久删除视频源及其所有相关数据，且不可撤销！
					</p>
				</div>

				<div class="space-y-2">
					<div class="flex items-center gap-2 text-sm">
						<span class="font-medium">类型：</span>
						<span class="px-2 py-1 bg-blue-100 text-blue-800 rounded text-xs">
							{getSourceTypeLabel(sourceType)}
						</span>
					</div>
					<div class="flex items-center gap-2 text-sm">
						<span class="font-medium">名称：</span>
						<span class="font-mono text-gray-800">"{sourceName}"</span>
					</div>
				</div>

				<div class="rounded-lg bg-yellow-50 p-3 border border-yellow-200">
					<div class="flex items-start gap-2">
						<svg class="h-4 w-4 text-yellow-600 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
						</svg>
						<div class="text-xs text-yellow-800">
							<p class="font-medium">将删除以下数据：</p>
							<ul class="mt-1 list-disc list-inside space-y-0.5 text-xs">
								<li>数据库中的视频源记录</li>
								<li>关联的视频信息</li>
								<li>视频分页数据</li>
								<li>下载历史记录</li>
							</ul>
						</div>
					</div>
				</div>

				<div class="space-y-3">
					<div class="flex items-start gap-3 p-3 rounded-lg border">
						<input 
							type="checkbox"
							id="delete-files" 
							bind:checked={deleteLocalFiles}
							class="mt-1 h-4 w-4 rounded border-gray-300 text-red-600 focus:ring-red-500"
						/>
						<div class="flex-1 space-y-1">
							<label for="delete-files" class="text-sm font-medium cursor-pointer flex items-center gap-2">
								<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
								</svg>
								同时删除本地文件
							</label>
							<p class="text-xs text-gray-600">
								勾选后将删除该视频源对应的所有本地下载文件和文件夹。
								<span class="text-red-600 font-medium">此操作不可恢复！</span>
							</p>
						</div>
					</div>

					{#if deleteLocalFiles}
						<div class="rounded-lg bg-red-50 p-3 border border-red-200">
							<p class="text-xs text-red-800 font-medium">⚠️ 文件删除警告</p>
							<p class="text-xs text-red-700 mt-1">
								选择删除本地文件后，该视频源下载的所有视频文件都将被永久删除，
								包括视频文件、字幕、封面图片等。请确保您有备份或不再需要这些文件。
							</p>
						</div>
					{/if}
				</div>

				<div class="space-y-2">
					<label for="confirm-input" class="text-sm font-medium text-gray-700">
						确认删除：请输入视频源名称 "<span class="font-mono text-red-600">{sourceName}</span>"
					</label>
					<input
						id="confirm-input"
						type="text"
						bind:value={confirmText}
						placeholder="输入视频源名称以确认删除"
						class="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-red-500 focus:border-red-500"
						disabled={isDeleting}
					/>
				</div>
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer class="flex justify-end gap-3 pt-4">
			<button
				type="button"
				class="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed"
				disabled={isDeleting}
				on:click={handleCancel}
			>
				取消
			</button>
			<button
				type="button"
				class="px-4 py-2 text-sm font-medium text-white bg-red-600 border border-transparent rounded-md hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500 disabled:opacity-50 disabled:cursor-not-allowed"
				disabled={!canConfirm || isDeleting}
				on:click={handleConfirm}
			>
				{#if isDeleting}
					<svg class="w-4 h-4 mr-2 animate-spin inline" fill="none" viewBox="0 0 24 24">
						<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
						<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
					</svg>
					删除中...
				{:else}
					确认删除
				{/if}
			</button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root> 