<script lang="ts">
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import PowerIcon from '@lucide/svelte/icons/power';
	import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import MoreVerticalIcon from '@lucide/svelte/icons/more-vertical';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
	import { FileText, ListTodo } from '@lucide/svelte';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { useSidebar } from '$lib/components/ui/sidebar/context.svelte.js';
	import { appStateStore, setVideoSourceFilter, clearAll, ToQuery } from '$lib/stores/filter';
	import { toast } from 'svelte-sonner';
	import DeleteVideoSourceDialog from './delete-video-source-dialog.svelte';
	import ResetPathDialog from './reset-path-dialog.svelte';

	import { type VideoSourcesResponse } from '$lib/types';
	import { VIDEO_SOURCES } from '$lib/consts';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
	import { goto } from '$app/navigation';
	import { videoSourceStore, setVideoSources } from '$lib/stores/video-source';
	import api from '$lib/api';
	const sidebar = useSidebar();

	const items = Object.values(VIDEO_SOURCES);

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

	// 展开的操作菜单状态 - 记录哪个视频源的操作菜单是展开的
	let expandedActionMenuKey = '';

	// 生成视频源的唯一key
	function getSourceKey(type: string, id: number) {
		return `${type}-${id}`;
	}

	function handleSourceClick(sourceType: string, sourceId: number) {
		setVideoSourceFilter(sourceType, sourceId.toString());
		goto(`/${ToQuery($appStateStore)}`);
		if (sidebar.isMobile) {
			sidebar.setOpenMobile(false);
		}
	}

	// 切换操作菜单的展开状态
	function toggleActionMenu(event: Event, type: string, id: number) {
		event.preventDefault();
		event.stopPropagation();
		const sourceKey = getSourceKey(type, id);
		expandedActionMenuKey = expandedActionMenuKey === sourceKey ? '' : sourceKey;
	}

	// 处理显示操作菜单（与MoreVerticalIcon的点击事件对应）
	function handleShowActions(
		event: Event,
		type: string,
		id: number,
		name: string,
		path: string,
		enabled: boolean
	) {
		event.preventDefault();
		event.stopPropagation();
		toggleActionMenu(event, type, id);
	}

	function handleLogoClick() {
		clearAll();
		goto('/');

		if (sidebar.isMobile) {
			sidebar.setOpenMobile(false);
		}
	}

	// 打开删除确认对话框
	function handleDeleteSource(
		event: Event,
		sourceType: string,
		sourceId: number,
		sourceName: string
	) {
		event.stopPropagation(); // 阻止触发父级的点击事件

		deleteSourceInfo = {
			type: sourceType,
			id: sourceId,
			name: sourceName
		};
		showDeleteDialog = true;
	}

	// 打开路径重设对话框
	function handleResetPath(
		event: Event,
		sourceType: string,
		sourceId: number,
		sourceName: string,
		currentPath: string
	) {
		event.stopPropagation(); // 阻止触发父级的点击事件

		resetPathSourceInfo = {
			type: sourceType,
			id: sourceId,
			name: sourceName,
			currentPath: currentPath
		};
		showResetPathDialog = true;
	}

	// 切换视频源启用状态
	async function handleToggleEnabled(
		event: Event,
		sourceType: string,
		sourceId: number,
		currentEnabled: boolean,
		sourceName: string
	) {
		event.stopPropagation(); // 阻止触发父级的点击事件

		try {
			const result = await api.updateVideoSourceEnabled(sourceType, sourceId, !currentEnabled);
			if (result.data.success) {
				toast.success(result.data.message);

				// 刷新视频源列表
				const response = await api.getVideoSources();
				setVideoSources(response.data);
			} else {
				toast.error('操作失败', { description: result.data.message });
			}
		} catch (error: any) {
			console.error('切换视频源状态失败:', error);
			toast.error('操作失败', { description: error.message });
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

				// 刷新视频源列表
				const response = await api.getVideoSources();
				setVideoSources(response.data);
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

				// 刷新视频源列表
				const response = await api.getVideoSources();
				setVideoSources(response.data);
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

	// 切换扫描已删除视频设置
	async function handleToggleScanDeleted(
		event: Event,
		sourceType: string,
		sourceId: number,
		currentScanDeleted: boolean,
		sourceName: string
	) {
		event.stopPropagation(); // 阻止触发父级的点击事件

		try {
			const newScanDeleted = !currentScanDeleted;
			const result = await api.updateVideoSourceScanDeleted(sourceType, sourceId, newScanDeleted);

			if (result.data.success) {
				toast.success('设置更新成功', {
					description: result.data.message
				});

				// 刷新视频源列表
				const response = await api.getVideoSources();
				setVideoSources(response.data);

				// 关闭操作菜单
				expandedActionMenuKey = '';
			} else {
				toast.error('设置更新失败', { description: result.data.message });
			}
		} catch (error: any) {
			console.error('设置更新失败:', error);
			toast.error('设置更新失败', { description: error.message });
		}
	}
</script>

<Sidebar.Root class="border-border bg-background border-r">
	<Sidebar.Header class="border-border flex h-[73px] items-center border-b">
		<a
			href="/"
			class="flex w-full items-center gap-3 px-4 py-3 hover:cursor-pointer"
			on:click={handleLogoClick}
		>
			<div class="flex h-8 w-8 items-center justify-center overflow-hidden rounded-lg">
				<img src="/favicon.png" alt="Bili Sync" class="h-6 w-6" />
			</div>
			<div class="grid flex-1 text-left text-sm leading-tight">
				<span class="truncate font-semibold">Bili Sync</span>
				<span class="text-muted-foreground truncate text-xs">视频管理系统</span>
			</div>
		</a>
	</Sidebar.Header>
	<Sidebar.Content class="flex flex-col px-2 py-3">
		<div class="flex-1">
			<Sidebar.Group>
				<Sidebar.GroupLabel
					class="text-muted-foreground mb-2 px-2 text-xs font-medium tracking-wider uppercase"
				>
					视频来源
				</Sidebar.GroupLabel>
				<Sidebar.GroupContent>
					<Sidebar.Menu class="space-y-1">
						{#each items as item (item.type)}
							<Collapsible.Root class="group/collapsible">
								<Sidebar.MenuItem>
									<Collapsible.Trigger class="w-full">
										{#snippet child({ props })}
											<Sidebar.MenuButton
												{...props}
												class="hover:bg-accent/50 text-foreground flex w-full cursor-pointer items-center justify-between rounded-lg px-3 py-2.5 font-medium transition-all duration-200"
											>
												<div class="flex flex-1 items-center gap-3">
													<item.icon class="text-muted-foreground h-4 w-4" />
													<span class="text-sm">{item.title}</span>
												</div>
												<ChevronRightIcon
													class="text-muted-foreground h-3 w-3 transition-transform duration-200 group-data-[state=open]/collapsible:rotate-90"
												/>
											</Sidebar.MenuButton>
										{/snippet}
									</Collapsible.Trigger>
									<Collapsible.Content class="mt-1">
										<div class="border-border ml-5 space-y-0.5 border-l pl-2">
											{#if $videoSourceStore}
												{#if $videoSourceStore[item.type as keyof VideoSourcesResponse]?.length > 0}
													{#each $videoSourceStore[item.type as keyof VideoSourcesResponse] as source (source.id)}
														<Sidebar.MenuItem>
															<div class="group/item flex items-start gap-1">
																<button
																	class="text-foreground hover:bg-accent/50 flex-1 cursor-pointer rounded-md px-3 py-2 text-left text-sm transition-all duration-200 {!source.enabled
																		? 'opacity-50'
																		: ''}"
																	on:click={() => handleSourceClick(item.type, source.id)}
																>
																	<div class="flex flex-col gap-1">
																		<span class="block leading-tight break-words"
																			>{source.name}</span
																		>
																		{#if !source.enabled}
																			<span class="text-muted-foreground text-xs">(已禁用)</span>
																		{/if}
																	</div>
																</button>
																<div class="pt-2">
																	<button
																		class="text-muted-foreground hover:text-foreground hover:bg-accent rounded p-1.5 opacity-0 transition-all duration-200 group-hover/item:opacity-100"
																		on:click={(e) =>
																			handleShowActions(
																				e,
																				item.type,
																				source.id,
																				source.name,
																				source.path,
																				source.enabled
																			)}
																		title="更多操作"
																	>
																		{#if expandedActionMenuKey === getSourceKey(item.type, source.id)}
																			<ChevronUpIcon class="h-3.5 w-3.5" />
																		{:else}
																			<MoreVerticalIcon class="h-3.5 w-3.5" />
																		{/if}
																	</button>
																</div>
															</div>

															<!-- 展开的操作菜单 -->
															{#if expandedActionMenuKey === getSourceKey(item.type, source.id)}
																<div class="border-border mt-2 ml-4 space-y-1 border-l pl-3">
																	<!-- 启用/禁用 -->
																	<button
																		class="hover:bg-accent/50 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs transition-colors"
																		on:click={(e) =>
																			handleToggleEnabled(
																				e,
																				item.type,
																				source.id,
																				source.enabled,
																				source.name
																			)}
																	>
																		<PowerIcon
																			class="h-3 w-3 {source.enabled
																				? 'text-green-600'
																				: 'text-gray-400'}"
																		/>
																		<span>{source.enabled ? '禁用' : '启用'}</span>
																	</button>

																	<!-- 重设路径 -->
																	<button
																		class="hover:bg-accent/50 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs transition-colors"
																		on:click={(e) =>
																			handleResetPath(
																				e,
																				item.type,
																				source.id,
																				source.name,
																				source.path
																			)}
																	>
																		<FolderOpenIcon class="h-3 w-3 text-orange-600" />
																		<span>重设路径</span>
																	</button>

																	<!-- 扫描已删除视频设置 -->
																	<button
																		class="hover:bg-accent/50 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs transition-colors"
																		on:click={(e) =>
																			handleToggleScanDeleted(
																				e,
																				item.type,
																				source.id,
																				source.scan_deleted_videos,
																				source.name
																			)}
																	>
																		<RotateCcwIcon
																			class="h-3 w-3 {source.scan_deleted_videos
																				? 'text-blue-600'
																				: 'text-gray-400'}"
																		/>
																		<span
																			>{source.scan_deleted_videos
																				? '禁用扫描已删除'
																				: '启用扫描已删除'}</span
																		>
																	</button>

																	<!-- 删除视频源 -->
																	<button
																		class="hover:bg-destructive/10 text-destructive flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs transition-colors"
																		on:click={(e) =>
																			handleDeleteSource(e, item.type, source.id, source.name)}
																	>
																		<TrashIcon class="h-3 w-3" />
																		<span>删除</span>
																	</button>
																</div>
															{/if}
														</Sidebar.MenuItem>
													{/each}
												{:else}
													<div class="text-muted-foreground px-3 py-2 text-sm">无数据</div>
												{/if}
											{:else}
												<div class="text-muted-foreground px-3 py-2 text-sm">加载中...</div>
											{/if}
										</div>
									</Collapsible.Content>
								</Sidebar.MenuItem>
							</Collapsible.Root>
						{/each}
					</Sidebar.Menu>
				</Sidebar.GroupContent>
			</Sidebar.Group>
		</div>

		<!-- 固定在底部的设置选项 -->
		<div class="border-border mt-auto border-t pt-4">
			<Sidebar.Menu class="space-y-1">
				<Sidebar.MenuItem>
					<Sidebar.MenuButton>
						<a
							href="/queue"
							class="hover:bg-accent/50 text-foreground flex w-full cursor-pointer items-center justify-between rounded-lg px-3 py-2.5 font-medium transition-all duration-200"
						>
							<div class="flex flex-1 items-center gap-3">
								<ListTodo class="text-muted-foreground h-4 w-4" />
								<span class="text-sm">任务队列</span>
							</div>
						</a>
					</Sidebar.MenuButton>
				</Sidebar.MenuItem>
				<Sidebar.MenuItem>
					<Sidebar.MenuButton>
						<a
							href="/logs"
							class="hover:bg-accent/50 text-foreground flex w-full cursor-pointer items-center justify-between rounded-lg px-3 py-2.5 font-medium transition-all duration-200"
						>
							<div class="flex flex-1 items-center gap-3">
								<FileText class="text-muted-foreground h-4 w-4" />
								<span class="text-sm">系统日志</span>
							</div>
						</a>
					</Sidebar.MenuButton>
				</Sidebar.MenuItem>
				<Sidebar.MenuItem>
					<Sidebar.MenuButton>
						<a
							href="/settings"
							class="hover:bg-accent/50 text-foreground flex w-full cursor-pointer items-center justify-between rounded-lg px-3 py-2.5 font-medium transition-all duration-200"
						>
							<div class="flex flex-1 items-center gap-3">
								<SettingsIcon class="text-muted-foreground h-4 w-4" />
								<span class="text-sm">设置</span>
							</div>
						</a>
					</Sidebar.MenuButton>
				</Sidebar.MenuItem>
			</Sidebar.Menu>
		</div>
	</Sidebar.Content>
</Sidebar.Root>

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
