<script lang="ts">
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { useSidebar } from '$lib/components/ui/sidebar/context.svelte.js';
	import { appStateStore, setVideoSourceFilter, clearAll, ToQuery } from '$lib/stores/filter';
	import { toast } from 'svelte-sonner';
	import DeleteVideoSourceDialog from './delete-video-source-dialog.svelte';

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

	function handleSourceClick(sourceType: string, sourceId: number) {
		setVideoSourceFilter(sourceType, sourceId.toString());
		goto(`/${ToQuery($appStateStore)}`);
		if (sidebar.isMobile) {
			sidebar.setOpenMobile(false);
		}
	}

	function handleLogoClick() {
		clearAll();
		goto('/');

		if (sidebar.isMobile) {
			sidebar.setOpenMobile(false);
		}
	}

	// 打开删除确认对话框
	function handleDeleteSource(event: Event, sourceType: string, sourceId: number, sourceName: string) {
		event.stopPropagation(); // 阻止触发父级的点击事件
		
		deleteSourceInfo = {
			type: sourceType,
			id: sourceId,
			name: sourceName
		};
		showDeleteDialog = true;
		}
		
	// 确认删除
	async function handleConfirmDelete(event: CustomEvent<{ deleteLocalFiles: boolean }>) {
		const { deleteLocalFiles } = event.detail;
		
		try {
			const result = await api.deleteVideoSource(deleteSourceInfo.type, deleteSourceInfo.id, deleteLocalFiles);
			if (result.data.success) {
				toast.success('删除成功', { 
					description: result.data.message + (deleteLocalFiles ? '，本地文件已删除' : '，本地文件已保留') 
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
															<div class="flex items-center gap-1 group/item">
																<button
																	class="text-foreground hover:bg-accent/50 flex-1 cursor-pointer rounded-md px-3 py-2 text-left text-sm transition-all duration-200"
																	on:click={() => handleSourceClick(item.type, source.id)}
																>
																	<span class="block truncate">{source.name}</span>
																</button>
																<button
																	class="text-muted-foreground hover:text-destructive hover:bg-destructive/10 opacity-0 group-hover/item:opacity-100 p-1.5 rounded transition-all duration-200"
																	on:click={(e) => handleDeleteSource(e, item.type, source.id, source.name)}
																	title="删除视频源"
																>
																	<TrashIcon class="h-3.5 w-3.5" />
																</button>
															</div>
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
 