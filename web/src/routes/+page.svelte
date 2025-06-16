<script lang="ts">
	import VideoCard from '$lib/components/video-card.svelte';
	import FilterBadge from '$lib/components/filter-badge.svelte';
	import Pagination from '$lib/components/pagination.svelte';
	import AuthLogin from '$lib/components/auth-login.svelte';
	import InitialSetup from '$lib/components/initial-setup.svelte';
	import api from '$lib/api';
	import type { VideosResponse, VideoSourcesResponse, ApiError } from '$lib/types';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { setVideoSources, videoSourceStore } from '$lib/stores/video-source';
	import { VIDEO_SOURCES } from '$lib/consts';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import {
		appStateStore,
		clearVideoSourceFilter,
		setQuery,
		setVideoSourceFilter,
		ToQuery
	} from '$lib/stores/filter';
	import { toast } from 'svelte-sonner';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';

	// 认证状态
	let isAuthenticated = false;
	let needsInitialSetup = false;
	let checkingSetup = true;
	let videosData: VideosResponse | null = null;
	let loading = false;
	let currentPage = 0;
	let pageSize = 20; // 改为可变的
	let currentFilter: { type: string; id: string } | null = null;
	let lastSearch: string | null = null;

	// 响应式变量
	let innerWidth = 0;
	let innerHeight = 0;

	// 动态计算每页数量
	function calculateOptimalPageSize(): number {
		if (innerWidth === 0 || innerHeight === 0) return 20;
		
		// 卡片最小宽度260px，间距16px
		const cardMinWidth = 260 + 16;
		const availableWidth = innerWidth - 300; // 减去侧边栏宽度
		const cardsPerRow = Math.floor(availableWidth / cardMinWidth);
		
		// 卡片高度约200px，间距16px
		const cardHeight = 200 + 16;
		const availableHeight = innerHeight - 200; // 减去头部和分页区域
		const rowsPerPage = Math.floor(availableHeight / cardHeight);
		
		const optimalSize = Math.max(cardsPerRow * rowsPerPage, 12); // 最少12个
		return Math.min(optimalSize, 100); // 最多100个
	}

	// 自动调整页面大小
	let autoPageSize = true;

	// 批量重置状态
	let resetAllDialogOpen = false;
	let resettingAll = false;
	
	// 批量重置选项
	let resetOptions = {
		all: true,           // 重置所有失败任务
		videoCover: false,   // 重置视频封面
		videoContent: false, // 重置视频内容
		videoInfo: false,    // 重置视频信息
		videoDanmaku: false, // 重置视频弹幕
		videoSubtitle: false // 重置视频字幕
	};

	// 处理登录成功
	function handleLoginSuccess(event: CustomEvent) {
		isAuthenticated = true;
		// 登录成功后加载数据
		loadInitialData();
	}

	// 处理初始设置完成
	function handleSetupComplete() {
		needsInitialSetup = false;
		checkingSetup = true;
		// 重新检查设置状态
		checkInitialSetup();
	}

	// 退出登录
	function logout() {
		isAuthenticated = false;
		api.setAuthToken('');
		videosData = null;
		toast.info('已退出登录');
	}

	// 检查是否需要初始设置
	async function checkInitialSetup() {
		try {
			// 暂时简化逻辑，检查本地是否有token
			const storedToken = localStorage.getItem('auth_token');
			
			if (!storedToken) {
				// 没有token，需要初始设置
				needsInitialSetup = true;
				checkingSetup = false;
				return;
			}

			// 有token，尝试验证
			api.setAuthToken(storedToken);
			try {
				await api.getVideoSources();
				isAuthenticated = true;
				loadInitialData();
			} catch (error) {
				// Token无效，需要重新登录
				isAuthenticated = false;
			}
		} catch (error) {
			console.error('检查初始设置失败:', error);
			// 如果检查失败，假设需要初始设置
			needsInitialSetup = true;
		} finally {
			checkingSetup = false;
		}
	}

	// 加载初始数据
	async function loadInitialData() {
		try {
			// 获取视频源
			const sources = await api.getVideoSources();
			setVideoSources(sources.data);

			// 加载视频列表
			handleSearchParamsChange();
		} catch (error) {
			console.error('加载数据失败:', error);
			toast.error('加载数据失败');
		}
	}

	// 从URL参数获取筛选条件
	function getFilterFromURL(searchParams: URLSearchParams) {
		for (const source of Object.values(VIDEO_SOURCES)) {
			const value = searchParams.get(source.type);
			if (value) {
				return { type: source.type, id: value };
			}
		}
		return null;
	}

	// 获取筛选项名称
	function getFilterName(type: string, id: string): string {
		const videoSources = $videoSourceStore;
		if (!videoSources || !type || !id) return '';

		const sources = videoSources[type as keyof VideoSourcesResponse];
		const source = sources?.find((s) => s.id.toString() === id);
		return source?.name || '';
	}

	// 获取筛选项标题
	function getFilterTitle(type: string): string {
		const sourceConfig = Object.values(VIDEO_SOURCES).find((s) => s.type === type);
		return sourceConfig?.title || '';
	}

	async function loadVideos(
		query?: string,
		pageNum: number = 0,
		filter?: { type: string; id: string } | null
	) {
		loading = true;
		try {
			const params: Record<string, string | number> = {
				page: pageNum,
				page_size: pageSize
			};

			if (query) {
				params.query = query;
			}

			// 添加筛选参数
			if (filter) {
				params[filter.type] = parseInt(filter.id);
			}

			const result = await api.getVideos(params);
			videosData = result.data;
			currentPage = pageNum;
		} catch (error) {
			console.error('加载视频失败:', error);
			toast.error('加载视频失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	async function handlePageChange(pageNum: number) {
		const query = ToQuery($appStateStore);
		if (query) {
			goto(`/${query}&page=${pageNum}`);
		} else {
			goto(`/?page=${pageNum}`);
		}
	}

	async function handleSearchParamsChange() {
		const query = $page.url.searchParams.get('query');
		currentFilter = getFilterFromURL($page.url.searchParams);
		setQuery(query || '');
		if (currentFilter) {
			setVideoSourceFilter(currentFilter.type, currentFilter.id);
		} else {
			clearVideoSourceFilter();
		}
		loadVideos(query || '', parseInt($page.url.searchParams.get('page') || '0'), currentFilter);
	}

	function handleFilterRemove() {
		clearVideoSourceFilter();
		goto(`/${ToQuery($appStateStore)}`);
	}

	// 批量重置所有视频
	async function handleResetAllVideos() {
		resettingAll = true;
		try {
			let result;
			
			if (resetOptions.all) {
				// 重置所有失败任务
				result = await api.resetAllVideos();
			} else {
				// 选择性重置特定任务
				const taskIndexes = [];
				
				// 根据选择的选项确定要重置的任务索引
				if (resetOptions.videoCover) taskIndexes.push(0);    // 视频封面
				if (resetOptions.videoContent) taskIndexes.push(1);  // 视频内容  
				if (resetOptions.videoInfo) taskIndexes.push(2);     // 视频信息
				if (resetOptions.videoDanmaku) taskIndexes.push(3);  // 视频弹幕
				if (resetOptions.videoSubtitle) taskIndexes.push(4); // 视频字幕
				
				if (taskIndexes.length === 0) {
					toast.error('请至少选择一个要重置的任务');
					return;
				}
				
				// 调用选择性重置API
				result = await api.resetSpecificTasks(taskIndexes);
			}
			
			const data = result.data;
			if (data.resetted) {
				toast.success('重置成功', {
					description: `已重置 ${data.resetted_videos_count} 个视频和 ${data.resetted_pages_count} 个分页`
				});
				// 重新加载当前页面数据
				handleSearchParamsChange();
			} else {
				toast.info('没有需要重置的视频');
			}
		} catch (error) {
			console.error('重置失败:', error);
			toast.error('重置失败', {
				description: (error as ApiError).message
			});
		} finally {
			resettingAll = false;
			resetAllDialogOpen = false;
		}
	}
	
	// 处理重置选项变化
	function handleResetOptionChange(option: string, checked: boolean) {
		if (option === 'all') {
			resetOptions.all = checked;
			if (checked) {
				// 选择"重置所有"时，取消其他选项
				resetOptions.videoCover = false;
				resetOptions.videoContent = false;
				resetOptions.videoInfo = false;
				resetOptions.videoDanmaku = false;
				resetOptions.videoSubtitle = false;
			}
		} else {
			// 选择具体任务时，取消"重置所有"
			resetOptions.all = false;
			if (option === 'videoCover') resetOptions.videoCover = checked;
			else if (option === 'videoContent') resetOptions.videoContent = checked;
			else if (option === 'videoInfo') resetOptions.videoInfo = checked;
			else if (option === 'videoDanmaku') resetOptions.videoDanmaku = checked;
			else if (option === 'videoSubtitle') resetOptions.videoSubtitle = checked;
		}
	}

	$: if ($page.url.search !== lastSearch) {
		lastSearch = $page.url.search;
		handleSearchParamsChange();
	}

	// 检查认证状态和初始设置
	onMount(async () => {
		setBreadcrumb([{ label: '主页', isActive: true }]);
		
		// 检查是否需要初始设置
		await checkInitialSetup();
	});

	$: totalPages = videosData ? Math.ceil(videosData.total_count / pageSize) : 0;
	$: filterTitle = currentFilter ? getFilterTitle(currentFilter.type) : '';
	$: filterName = currentFilter ? getFilterName(currentFilter.type, currentFilter.id) : '';

	// 响应式调整页面大小
	$: if (autoPageSize && innerWidth > 0 && innerHeight > 0) {
		const newPageSize = calculateOptimalPageSize();
		if (newPageSize !== pageSize) {
			pageSize = newPageSize;
			// 重新加载当前页面
			if (isAuthenticated && videosData) {
				handleSearchParamsChange();
			}
		}
	}
</script>

<svelte:head>
	<title>主页 - Bili Sync</title>
</svelte:head>

<svelte:window bind:innerWidth bind:innerHeight />

{#if checkingSetup}
	<div class="flex min-h-screen items-center justify-center bg-gray-50">
		<div class="text-center">
			<div class="mb-4 text-lg">正在检查系统状态...</div>
			<div class="text-sm text-gray-600">请稍候</div>
		</div>
	</div>
{:else if needsInitialSetup}
	<InitialSetup on:setup-complete={handleSetupComplete} />
{:else if !isAuthenticated}
	<AuthLogin on:login-success={handleLoginSuccess} />
{:else}
	<FilterBadge {filterTitle} {filterName} onRemove={handleFilterRemove} />

	<!-- 统计信息和操作按钮 -->
	{#if videosData}
		<div class="mb-6 space-y-4 sm:space-y-0 sm:flex sm:items-center sm:justify-between">
			<!-- 统计信息 -->
			<div class="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4">
				<div class="text-muted-foreground text-sm">
					共 {videosData.total_count} 个视频，{totalPages} 页
				</div>
				<div class="text-muted-foreground text-xs">
					每页 {pageSize} 个
				</div>
			</div>
			
			<!-- 操作按钮区域 -->
			<div class="flex flex-col sm:flex-row items-start sm:items-center gap-3 sm:gap-2">
				<!-- 页面大小控制 -->
				<div class="flex items-center gap-2">
					<label for="page-size-select" class="text-muted-foreground text-sm whitespace-nowrap">每页:</label>
					<select
						id="page-size-select"
						value={autoPageSize ? 'auto' : pageSize}
						on:change={(e) => {
							const value = e.currentTarget.value;
							if (value === 'auto') {
								autoPageSize = true;
								pageSize = calculateOptimalPageSize();
							} else {
								autoPageSize = false;
								pageSize = parseInt(value);
							}
							currentPage = 0;
							handleSearchParamsChange();
						}}
						class="border-input bg-background h-8 rounded-md border px-2 py-1 text-sm min-w-[80px]"
					>
						<option value="auto">自动</option>
						<option value={12}>12</option>
						<option value={20}>20</option>
						<option value={30}>30</option>
						<option value={50}>50</option>
						<option value={100}>100</option>
					</select>
				</div>
				
				<!-- 强制重置按钮 -->
				<Button
					size="sm"
					variant="outline"
					onclick={() => (resetAllDialogOpen = true)}
					disabled={resettingAll}
					class="w-full sm:w-auto"
				>
					<RotateCcwIcon class="mr-2 h-4 w-4 {resettingAll ? 'animate-spin' : ''}" />
					强制重置
				</Button>
			</div>
		</div>
	{/if}

	<!-- 视频卡片网格 -->
	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-muted-foreground">加载中...</div>
		</div>
	{:else if videosData?.videos.length}
		<div
			style="display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 16px; width: 100%; max-width: none; justify-items: start;"
		>
			{#each videosData.videos as video (video.id)}
				<div style="max-width: 400px; width: 100%;">
					<VideoCard {video} />
				</div>
			{/each}
		</div>

		<!-- 翻页组件 -->
		<Pagination {currentPage} {totalPages} onPageChange={handlePageChange} />
	{:else}
		<div class="flex items-center justify-center py-12">
			<div class="space-y-2 text-center">
				<p class="text-muted-foreground">暂无视频数据</p>
				<p class="text-muted-foreground text-sm">尝试搜索或检查视频来源配置</p>
			</div>
		</div>
	{/if}

	<!-- 批量重置确认对话框 -->
	<AlertDialog.Root bind:open={resetAllDialogOpen}>
		<AlertDialog.Content class="max-w-md">
			<AlertDialog.Header>
				<AlertDialog.Title>强制批量重置</AlertDialog.Title>
				<AlertDialog.Description>
					<p class="mb-4">选择要强制重置的任务类型（不管当前状态）：</p>
				</AlertDialog.Description>
			</AlertDialog.Header>
			
			<div class="space-y-4">
				<!-- 重置所有失败任务 -->
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="reset-all"
						bind:checked={resetOptions.all}
						on:change={(e) => handleResetOptionChange('all', e.currentTarget.checked)}
						class="h-4 w-4 rounded border-gray-300"
					/>
					<Label for="reset-all" class="text-sm font-medium">
						强制重置所有任务
					</Label>
				</div>
				
				<div class="border-t pt-3">
					<p class="text-sm text-muted-foreground mb-3">或选择特定任务：</p>
					
					<!-- 视频封面 -->
					<div class="flex items-center space-x-2 mb-2">
						<input
							type="checkbox"
							id="reset-cover"
							bind:checked={resetOptions.videoCover}
							on:change={(e) => handleResetOptionChange('videoCover', e.currentTarget.checked)}
							class="h-4 w-4 rounded border-gray-300"
						/>
						<Label for="reset-cover" class="text-sm">
							强制重置视频封面
						</Label>
					</div>
					
					<!-- 视频内容 -->
					<div class="flex items-center space-x-2 mb-2">
						<input
							type="checkbox"
							id="reset-content"
							bind:checked={resetOptions.videoContent}
							on:change={(e) => handleResetOptionChange('videoContent', e.currentTarget.checked)}
							class="h-4 w-4 rounded border-gray-300"
						/>
						<Label for="reset-content" class="text-sm">
							强制重置视频内容
						</Label>
					</div>
					
					<!-- 视频信息 -->
					<div class="flex items-center space-x-2 mb-2">
						<input
							type="checkbox"
							id="reset-info"
							bind:checked={resetOptions.videoInfo}
							on:change={(e) => handleResetOptionChange('videoInfo', e.currentTarget.checked)}
							class="h-4 w-4 rounded border-gray-300"
						/>
						<Label for="reset-info" class="text-sm">
							强制重置视频信息
						</Label>
					</div>
					
					<!-- 视频弹幕 -->
					<div class="flex items-center space-x-2 mb-2">
						<input
							type="checkbox"
							id="reset-danmaku"
							bind:checked={resetOptions.videoDanmaku}
							on:change={(e) => handleResetOptionChange('videoDanmaku', e.currentTarget.checked)}
							class="h-4 w-4 rounded border-gray-300"
						/>
						<Label for="reset-danmaku" class="text-sm">
							强制重置视频弹幕
						</Label>
					</div>
					
					<!-- 视频字幕 -->
					<div class="flex items-center space-x-2 mb-2">
						<input
							type="checkbox"
							id="reset-subtitle"
							bind:checked={resetOptions.videoSubtitle}
							on:change={(e) => handleResetOptionChange('videoSubtitle', e.currentTarget.checked)}
							class="h-4 w-4 rounded border-gray-300"
						/>
						<Label for="reset-subtitle" class="text-sm">
							强制重置视频字幕
						</Label>
					</div>
				</div>
				
				<div class="bg-yellow-50 border border-yellow-200 rounded-lg p-3">
					<p class="text-sm text-yellow-800">
						<strong>注意：</strong>强制重置会将选中的任务状态重置为"未开始"，不管当前是否已完成。选择特定任务重置时，会同时重置对应的分P下载状态。
					</p>
				</div>
			</div>
			
			<AlertDialog.Footer>
				<AlertDialog.Cancel>取消</AlertDialog.Cancel>
				<AlertDialog.Action onclick={handleResetAllVideos} disabled={resettingAll}>
					{resettingAll ? '重置中...' : '确认重置'}
				</AlertDialog.Action>
			</AlertDialog.Footer>
		</AlertDialog.Content>
	</AlertDialog.Root>
{/if}
