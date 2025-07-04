<script lang="ts">
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Button } from '$lib/components/ui/button';
	import { createEventDispatcher } from 'svelte';
	import { api } from '$lib/api';
	import type { SubmissionVideoInfo, SubmissionVideosResponse } from '$lib/types';
	
	export let isOpen = false;
	export let upId = '';
	export let upName = '';
	
	const dispatch = createEventDispatcher<{
		confirm: string[];
		cancel: void;
	}>();
	
	let videos: SubmissionVideoInfo[] = [];
	let selectedVideos: Set<string> = new Set();
	let isLoading = false;
	let error: string | null = null;
	let currentPage = 1;
	let totalCount = 0;
	let searchQuery = '';
	let filteredVideos: SubmissionVideoInfo[] = [];
	
	const PAGE_SIZE = 20;
	
	// 重置状态
	function resetState() {
		videos = [];
		selectedVideos = new Set();
		isLoading = false;
		error = null;
		currentPage = 1;
		totalCount = 0;
		searchQuery = '';
		filteredVideos = [];
	}
	
	// 当对话框打开时加载数据
	$: if (isOpen && upId) {
		resetState();
		loadVideos();
	}
	
	// 搜索过滤
	$: {
		if (searchQuery.trim()) {
			filteredVideos = videos.filter(video => 
				video.title.toLowerCase().includes(searchQuery.toLowerCase().trim())
			);
		} else {
			filteredVideos = videos;
		}
	}
	
	// 加载UP主投稿列表
	async function loadVideos() {
		if (!upId) return;
		
		isLoading = true;
		error = null;
		
		try {
			const response = await api.getSubmissionVideos({
				up_id: upId,
				page: currentPage,
				page_size: PAGE_SIZE
			});
			
			if (response.data && response.data.videos) {
				if (currentPage === 1) {
					videos = response.data.videos;
				} else {
					videos = [...videos, ...response.data.videos];
				}
				totalCount = response.data.total;
			} else {
				error = '获取投稿列表失败';
			}
		} catch (err) {
			error = err instanceof Error ? err.message : '网络请求失败';
		} finally {
			isLoading = false;
		}
	}
	
	// 加载更多
	async function loadMore() {
		if (isLoading || videos.length >= totalCount) return;
		currentPage++;
		await loadVideos();
	}
	
	// 处理视频选择
	function toggleVideo(bvid: string) {
		if (selectedVideos.has(bvid)) {
			selectedVideos.delete(bvid);
		} else {
			selectedVideos.add(bvid);
		}
		selectedVideos = selectedVideos; // 触发响应式更新
	}
	
	// 全选
	function selectAll() {
		filteredVideos.forEach(video => selectedVideos.add(video.bvid));
		selectedVideos = selectedVideos;
	}
	
	// 全不选
	function selectNone() {
		filteredVideos.forEach(video => selectedVideos.delete(video.bvid));
		selectedVideos = selectedVideos;
	}
	
	// 反选
	function invertSelection() {
		filteredVideos.forEach(video => {
			if (selectedVideos.has(video.bvid)) {
				selectedVideos.delete(video.bvid);
			} else {
				selectedVideos.add(video.bvid);
			}
		});
		selectedVideos = selectedVideos;
	}
	
	// 处理确认
	function handleConfirm() {
		dispatch('confirm', Array.from(selectedVideos));
		isOpen = false;
	}
	
	// 处理取消
	function handleCancel() {
		dispatch('cancel');
		isOpen = false;
	}
	
	// 格式化时间
	function formatDate(pubtime: string): string {
		try {
			return new Date(pubtime).toLocaleDateString('zh-CN');
		} catch (e) {
			return pubtime;
		}
	}
	
	// 格式化播放量
	function formatPlayCount(count: number): string {
		if (count >= 10000) {
			return (count / 10000).toFixed(1) + '万';
		}
		return count.toString();
	}

	// 处理B站图片URL
	function processBilibiliImageUrl(url: string): string {
		if (!url) return '';

		if (url.startsWith('https://')) return url;
		if (url.startsWith('//')) return 'https:' + url;
		if (url.startsWith('http://')) return url.replace('http://', 'https://');
		if (!url.startsWith('http')) return 'https://' + url;

		return url.split('@')[0];
	}

	// 处理图片加载错误
	function handleImageError(event: Event) {
		const img = event.target as HTMLImageElement;
		img.style.display = 'none';
		const parent = img.parentElement;
		if (parent && !parent.querySelector('.placeholder')) {
			const placeholder = document.createElement('div');
			placeholder.className = 'h-16 w-28 bg-gray-200 rounded flex items-center justify-center text-xs text-gray-400 flex-shrink-0 placeholder';
			placeholder.textContent = '无封面';
			parent.appendChild(placeholder);
		}
	}
	
	// 计算已选择的视频数量
	$: selectedCount = Array.from(selectedVideos).filter(bvid => 
		filteredVideos.some(video => video.bvid === bvid)
	).length;
	
	$: canLoadMore = videos.length < totalCount && !isLoading;
</script>

<AlertDialog.Root bind:open={isOpen}>
	<AlertDialog.Content class="max-w-4xl max-h-[80vh] overflow-hidden flex flex-col">
		<AlertDialog.Header class="flex-shrink-0">
			<AlertDialog.Title class="flex items-center gap-2 text-blue-600">
				<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M7 4V2a1 1 0 011-1h8a1 1 0 011 1v2h4a1 1 0 110 2h-1v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6H3a1 1 0 110-2h4zM6 6v14h12V6H6zm3-2V3h6v1H9z"
					/>
				</svg>
				选择历史投稿
			</AlertDialog.Title>
			<AlertDialog.Description class="space-y-4">
				<div class="rounded-lg border border-blue-200 bg-blue-50 p-3">
					<p class="text-sm font-medium text-blue-800">📹 投稿选择说明</p>
					<p class="mt-1 text-xs text-blue-700">
						选择您希望下载的历史投稿。未选择的视频将不会被下载，但会在数据库中记录信息。新发布的投稿会自动下载。
					</p>
				</div>
				
				<div class="flex items-center gap-2 text-sm">
					<span class="font-medium">UP主：</span>
					<span class="font-mono text-gray-800">"{upName || `UP主 ${upId}`}"</span>
					<span class="text-gray-500">({upId})</span>
				</div>
			</AlertDialog.Description>
		</AlertDialog.Header>
		
		<div class="flex-1 overflow-hidden flex flex-col min-h-0">
			{#if error}
				<div class="rounded-lg border border-red-200 bg-red-50 p-4">
					<div class="flex items-center gap-2">
						<svg class="h-5 w-5 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
							/>
						</svg>
						<span class="text-sm font-medium text-red-800">加载失败</span>
					</div>
					<p class="mt-1 text-sm text-red-700">{error}</p>
					<button
						type="button"
						class="mt-2 text-sm text-red-600 underline hover:text-red-800"
						onclick={loadVideos}
					>
						重试
					</button>
				</div>
			{:else}
				<!-- 搜索和操作栏 -->
				<div class="space-y-3 mb-4 flex-shrink-0">
					<div class="flex gap-2">
						<input
							type="text"
							bind:value={searchQuery}
							placeholder="搜索视频标题..."
							class="flex-1 rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:ring-2 focus:ring-blue-500 focus:outline-none"
						/>
					</div>
					
					<div class="flex items-center justify-between">
						<div class="flex gap-2">
							<button
								type="button"
								class="rounded-md border border-gray-300 bg-white px-3 py-1 text-sm font-medium text-gray-700 hover:bg-gray-50"
								onclick={selectAll}
								disabled={filteredVideos.length === 0}
							>
								全选
							</button>
							<button
								type="button"
								class="rounded-md border border-gray-300 bg-white px-3 py-1 text-sm font-medium text-gray-700 hover:bg-gray-50"
								onclick={selectNone}
								disabled={selectedCount === 0}
							>
								全不选
							</button>
							<button
								type="button"
								class="rounded-md border border-gray-300 bg-white px-3 py-1 text-sm font-medium text-gray-700 hover:bg-gray-50"
								onclick={invertSelection}
								disabled={filteredVideos.length === 0}
							>
								反选
							</button>
						</div>
						
						<div class="text-sm text-gray-600">
							已选择 {selectedCount} / {filteredVideos.length} 个视频
						</div>
					</div>
				</div>
				
				<!-- 视频列表 -->
				<div class="flex-1 overflow-y-auto min-h-0">
					{#if isLoading && videos.length === 0}
						<div class="flex items-center justify-center py-8">
							<svg class="h-8 w-8 animate-spin text-blue-600" fill="none" viewBox="0 0 24 24">
								<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
								<path
									class="opacity-75"
									fill="currentColor"
									d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
								></path>
							</svg>
							<span class="ml-2 text-sm text-gray-600">加载中...</span>
						</div>
					{:else if filteredVideos.length === 0}
						<div class="flex flex-col items-center justify-center py-8 text-gray-500">
							<svg class="h-12 w-12 mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
								/>
							</svg>
							<p class="text-sm">没有找到视频</p>
						</div>
					{:else}
						<div class="space-y-2">
							{#each filteredVideos as video (video.bvid)}
								<div
									class="flex items-center gap-3 rounded-lg border p-3 hover:bg-gray-50 {selectedVideos.has(video.bvid) ? 'border-blue-300 bg-blue-50' : 'border-gray-200'}"
								>
									<input
										type="checkbox"
										checked={selectedVideos.has(video.bvid)}
										onchange={() => toggleVideo(video.bvid)}
										class="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
									/>
									
									<img
										src={processBilibiliImageUrl(video.cover)}
										alt={video.title}
										class="h-16 w-28 rounded object-cover flex-shrink-0"
										loading="lazy"
										crossorigin="anonymous"
										referrerpolicy="no-referrer"
										onerror={handleImageError}
									/>
									
									<div class="flex-1 min-w-0">
										<h4 class="text-sm font-medium text-gray-900 line-clamp-2">
											{video.title}
										</h4>
										<p class="mt-1 text-xs text-gray-600 line-clamp-2">
											{video.description || '无简介'}
										</p>
										<div class="mt-2 flex items-center gap-4 text-xs text-gray-500">
											<span>🎬 {formatPlayCount(video.view)}</span>
											<span>💬 {formatPlayCount(video.danmaku)}</span>
											<span>📅 {formatDate(video.pubtime)}</span>
											<span class="font-mono">{video.bvid}</span>
										</div>
									</div>
								</div>
							{/each}
							
							{#if canLoadMore}
								<div class="flex justify-center py-4">
									<button
										type="button"
										class="rounded-md border border-blue-300 bg-blue-50 px-4 py-2 text-sm font-medium text-blue-700 hover:bg-blue-100 disabled:cursor-not-allowed disabled:opacity-50"
										disabled={isLoading}
										onclick={loadMore}
									>
										{#if isLoading}
											<svg class="mr-2 inline h-4 w-4 animate-spin" fill="none" viewBox="0 0 24 24">
												<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
												<path
													class="opacity-75"
													fill="currentColor"
													d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
												></path>
											</svg>
											加载中...
										{:else}
											加载更多 ({videos.length}/{totalCount})
										{/if}
									</button>
								</div>
							{:else if videos.length > 0 && videos.length < totalCount}
								<div class="text-center py-4 text-sm text-gray-500">
									已加载全部 {totalCount} 个视频
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{/if}
		</div>
		
		<AlertDialog.Footer class="flex justify-end gap-3 pt-4 flex-shrink-0">
			<button
				type="button"
				class="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:outline-none"
				onclick={handleCancel}
			>
				取消
			</button>
			<button
				type="button"
				class="rounded-md border border-transparent bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:outline-none"
				onclick={handleConfirm}
			>
				确认选择 ({selectedVideos.size} 个视频)
			</button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

<style>
	.line-clamp-2 {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	:global(.placeholder) {
		flex-shrink: 0;
	}
</style>