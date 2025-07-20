<script lang="ts">
	import { onMount } from 'svelte';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { toast } from 'svelte-sonner';
	import api from '$lib/api';
	import type { VideoSourcesResponse, ApiError } from '$lib/types';
	import { VIDEO_SOURCES } from '$lib/consts';
	import { videoSourceStore, setVideoSources } from '$lib/stores/video-source';
	import DeleteVideoSourceDialog from '$lib/components/delete-video-source-dialog.svelte';
	import ResetPathDialog from '$lib/components/reset-path-dialog.svelte';
	
	// 图标导入
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import PowerIcon from '@lucide/svelte/icons/power';
	import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
	import { goto } from '$app/navigation';

	let loading = false;

	// 删除对话框状态
	let showDeleteDialog = false;
	let deleteSourceInfo = {
		type: '',
		id: 0,
		name: ''
	};

	// 路径重设对话框状态
	let showResetPathDialog = false;
	let resetPathSourceInfo = {
		type: '',
		id: 0,
		name: '',
		currentPath: ''
	};

	async function loadVideoSources() {
		loading = true;
		try {
			const response = await api.getVideoSources();
			setVideoSources(response.data);
		} catch (error) {
			console.error('加载视频源失败:', error);
			toast.error('加载视频源失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	// 切换视频源启用状态
	async function handleToggleEnabled(
		sourceType: string,
		sourceId: number,
		currentEnabled: boolean,
		sourceName: string
	) {
		try {
			const result = await api.updateVideoSourceEnabled(sourceType, sourceId, !currentEnabled);
			if (result.data.success) {
				toast.success(result.data.message);
				await loadVideoSources();
			} else {
				toast.error('操作失败', { description: result.data.message });
			}
		} catch (error: any) {
			console.error('切换视频源状态失败:', error);
			toast.error('操作失败', { description: error.message });
		}
	}

	// 打开删除确认对话框
	function handleDeleteSource(sourceType: string, sourceId: number, sourceName: string) {
		deleteSourceInfo = {
			type: sourceType,
			id: sourceId,
			name: sourceName
		};
		showDeleteDialog = true;
	}

	// 打开路径重设对话框
	function handleResetPath(
		sourceType: string,
		sourceId: number,
		sourceName: string,
		currentPath: string
	) {
		resetPathSourceInfo = {
			type: sourceType,
			id: sourceId,
			name: sourceName,
			currentPath: currentPath
		};
		showResetPathDialog = true;
	}

	// 切换扫描已删除视频设置
	async function handleToggleScanDeleted(
		sourceType: string,
		sourceId: number,
		currentScanDeleted: boolean
	) {
		try {
			const newScanDeleted = !currentScanDeleted;
			const result = await api.updateVideoSourceScanDeleted(sourceType, sourceId, newScanDeleted);

			if (result.data.success) {
				toast.success('设置更新成功', {
					description: result.data.message
				});
				await loadVideoSources();
			} else {
				toast.error('设置更新失败', { description: result.data.message });
			}
		} catch (error: any) {
			console.error('设置更新失败:', error);
			toast.error('设置更新失败', { description: error.message });
		}
	}

	// 确认删除
	async function handleConfirmDelete(event: CustomEvent<{ deleteLocalFiles: boolean }>) {
		const { deleteLocalFiles } = event.detail;

		try {
			const result = await api.deleteVideoSource(
				deleteSourceInfo.type,
				deleteSourceInfo.id,
				deleteLocalFiles
			);
			if (result.data.success) {
				toast.success('删除成功', {
					description:
						result.data.message + (deleteLocalFiles ? '，本地文件已删除' : '，本地文件已保留')
				});
				await loadVideoSources();
			} else {
				toast.error('删除失败', { description: result.data.message });
			}
		} catch (error: any) {
			console.error('删除视频源失败:', error);
			toast.error('删除失败', { description: error.message });
		}
	}

	// 取消删除
	function handleCancelDelete() {
		showDeleteDialog = false;
	}

	// 确认路径重设
	async function handleConfirmResetPath(
		event: CustomEvent<{
			new_path: string;
			apply_rename_rules?: boolean;
			clean_empty_folders?: boolean;
		}>
	) {
		const request = event.detail;

		try {
			const result = await api.resetVideoSourcePath(
				resetPathSourceInfo.type,
				resetPathSourceInfo.id,
				request
			);
			if (result.data.success) {
				toast.success('路径重设成功', {
					description:
						result.data.message +
						(request.apply_rename_rules ? `，已移动 ${result.data.moved_files_count} 个文件` : '')
				});
				await loadVideoSources();
			} else {
				toast.error('路径重设失败', { description: result.data.message });
			}
		} catch (error: any) {
			console.error('路径重设失败:', error);
			toast.error('路径重设失败', { description: error.message });
		}
	}

	// 取消路径重设
	function handleCancelResetPath() {
		showResetPathDialog = false;
	}

	function navigateToAddSource() {
		goto('/add-source');
	}

	onMount(() => {
		setBreadcrumb([{ label: '视频源管理' }]);
		loadVideoSources();
	});
</script>

<svelte:head>
	<title>视频源管理 - Bili Sync</title>
</svelte:head>

<div class="space-y-6">
	<!-- 页面头部 -->
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold">视频源管理</h1>
			<p class="text-muted-foreground">管理和配置您的视频源，包括收藏夹、合集、投稿和稍后再看</p>
		</div>
		<Button onclick={navigateToAddSource} class="flex items-center gap-2">
			<PlusIcon class="h-4 w-4" />
			添加视频源
		</Button>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-muted-foreground">加载中...</div>
		</div>
	{:else}
		
		<!-- 视频源分类展示 -->
		<div class="grid gap-6">
			{#each Object.entries(VIDEO_SOURCES) as [sourceKey, sourceConfig]}
				{@const sources = $videoSourceStore ? $videoSourceStore[sourceConfig.type] : []}
				<Card>
					<CardHeader>
						<CardTitle class="flex items-center gap-2">
							<sourceConfig.icon class="h-5 w-5" />
							{sourceConfig.title}
							<Badge variant="outline" class="ml-auto">
								{sources?.length || 0} 个
							</Badge>
						</CardTitle>
					</CardHeader>
					<CardContent>
						{#if sources && sources.length > 0}
							<div class="space-y-3">
								{#each sources as source}
									<div class="flex items-center justify-between p-3 border rounded-lg">
										<div class="flex-1 min-w-0">
											<div class="flex items-center gap-2 mb-1">
												<span class="font-medium truncate">{source.name}</span>
												<Badge variant={source.enabled ? 'default' : 'secondary'} class="text-xs">
													{source.enabled ? '已启用' : '已禁用'}
												</Badge>
											</div>
											<div class="text-sm text-muted-foreground truncate" title={source.path}>
												{source.path || '未设置路径'}
											</div>
											{#if source.scan_deleted_videos}
												<div class="text-xs text-blue-600 mt-1">扫描删除视频已启用</div>
											{/if}
										</div>
										
										<div class="flex items-center gap-1 ml-4">
											<!-- 启用/禁用 -->
											<Button
												size="sm"
												variant="ghost"
												onclick={() => handleToggleEnabled(sourceConfig.type, source.id, source.enabled, source.name)}
												title={source.enabled ? '禁用' : '启用'}
											>
												<PowerIcon class="h-4 w-4 {source.enabled ? 'text-green-600' : 'text-gray-400'}" />
											</Button>
											
											<!-- 重设路径 -->
											<Button
												size="sm"
												variant="ghost"
												onclick={() => handleResetPath(sourceConfig.type, source.id, source.name, source.path)}
												title="重设路径"
											>
												<FolderOpenIcon class="h-4 w-4 text-orange-600" />
											</Button>
											
											<!-- 扫描删除视频设置 -->
											<Button
												size="sm"
												variant="ghost"
												onclick={() => handleToggleScanDeleted(sourceConfig.type, source.id, source.scan_deleted_videos)}
												title={source.scan_deleted_videos ? '禁用扫描已删除' : '启用扫描已删除'}
											>
												<RotateCcwIcon class="h-4 w-4 {source.scan_deleted_videos ? 'text-blue-600' : 'text-gray-400'}" />
											</Button>
											
											<!-- 删除 -->
											<Button
												size="sm"
												variant="ghost"
												onclick={() => handleDeleteSource(sourceConfig.type, source.id, source.name)}
												title="删除"
											>
												<TrashIcon class="h-4 w-4 text-destructive" />
											</Button>
										</div>
									</div>
								{/each}
							</div>
						{:else}
							<div class="flex flex-col items-center justify-center py-8 text-center">
								<sourceConfig.icon class="h-12 w-12 text-muted-foreground mb-4" />
								<div class="text-muted-foreground mb-2">暂无{sourceConfig.title}</div>
								<p class="text-sm text-muted-foreground mb-4">
									{#if sourceConfig.type === 'favorite'}
										还没有添加任何收藏夹订阅
									{:else if sourceConfig.type === 'collection'}
										还没有添加任何合集或列表订阅
									{:else if sourceConfig.type === 'submission'}
										还没有添加任何用户投稿订阅
									{:else}
										还没有添加稍后再看订阅
									{/if}
								</p>
								<Button size="sm" variant="outline" onclick={navigateToAddSource}>
									<PlusIcon class="h-4 w-4 mr-2" />
									添加{sourceConfig.title}
								</Button>
							</div>
						{/if}
					</CardContent>
				</Card>
			{/each}
		</div>
	{/if}
</div>

<!-- 删除确认对话框 -->
<DeleteVideoSourceDialog
	bind:isOpen={showDeleteDialog}
	sourceName={deleteSourceInfo.name}
	sourceType={deleteSourceInfo.type}
	on:confirm={handleConfirmDelete}
	on:cancel={handleCancelDelete}
/>

<!-- 路径重设对话框 -->
<ResetPathDialog
	bind:isOpen={showResetPathDialog}
	sourceName={resetPathSourceInfo.name}
	sourceType={resetPathSourceInfo.type}
	currentPath={resetPathSourceInfo.currentPath}
	on:confirm={handleConfirmResetPath}
	on:cancel={handleCancelResetPath}
/>