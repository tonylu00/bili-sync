<script lang="ts">
	import VideoCard from '$lib/components/video-card.svelte';
	import FilterBadge from '$lib/components/filter-badge.svelte';
	import Pagination from '$lib/components/pagination.svelte';
	import AuthLogin from '$lib/components/auth-login.svelte';
	import InitialSetup from '$lib/components/initial-setup.svelte';
	import api from '$lib/api';
	import type { VideosResponse, VideoSourcesResponse, ApiError, TaskControlStatusResponse } from '$lib/types';
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
	import PauseIcon from '@lucide/svelte/icons/pause';
	import PlayIcon from '@lucide/svelte/icons/play';
	import FilterIcon from '@lucide/svelte/icons/filter';
	import { onDestroy } from 'svelte';

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

	// 任务控制状态
	let taskControlStatus: TaskControlStatusResponse | null = null;
	let loadingTaskControl = false;
	let statusUpdateInterval: ReturnType<typeof setInterval> | null = null;

	// 失败任务筛选状态
	let showFailedOnly = false;

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
			// 检查本地是否有token
			const storedToken = localStorage.getItem('auth_token');
			
			if (!storedToken) {
				// 没有token，检查是否是全新系统还是新浏览器
				try {
					const setupCheck = await api.checkInitialSetup();
					if (setupCheck.data.needs_setup) {
						// 全新系统，显示初始设置
						needsInitialSetup = true;
					} else {
						// 系统已配置但新浏览器，显示登录界面
						needsInitialSetup = false;
						isAuthenticated = false;
					}
				} catch (error) {
					// 无法连接后端，显示初始设置
					console.log('无法检查后端状态，显示初始设置:', error);
					needsInitialSetup = true;
				}
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
				// Token无效，清除无效token
				localStorage.removeItem('auth_token');
				api.setAuthToken('');
				
				// 检查是否是系统问题还是token问题
				try {
					const setupCheck = await api.checkInitialSetup();
					if (setupCheck.data.needs_setup) {
						// 系统未配置，显示初始设置
						needsInitialSetup = true;
					} else {
						// 系统已配置但token无效，显示登录界面
						needsInitialSetup = false;
						isAuthenticated = false;
					}
				} catch (apiError) {
					// 无法检查后端状态，显示登录界面
					needsInitialSetup = false;
					isAuthenticated = false;
				}
			}
		} catch (error) {
			console.error('检查初始设置失败:', error);
			// 如果检查失败，显示登录界面（比较安全的选择）
			needsInitialSetup = false;
			isAuthenticated = false;
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

			// 加载任务控制状态
			await loadTaskControlStatus();

			// 启动定时更新任务状态（每5秒更新一次）
			if (!statusUpdateInterval) {
				statusUpdateInterval = setInterval(async () => {
					if (isAuthenticated) {
						await loadTaskControlStatus();
					}
				}, 5000);
			}

			// 加载视频列表
			handleSearchParamsChange();
		} catch (error) {
			console.error('加载数据失败:', error);
			toast.error('加载数据失败');
		}
	}

	// 加载任务控制状态
	async function loadTaskControlStatus() {
		try {
			const response = await api.getTaskControlStatus();
			taskControlStatus = response.data;
		} catch (error) {
			console.error('获取任务控制状态失败:', error);
		}
	}

	// 暂停所有任务
	async function pauseAllTasks() {
		if (loadingTaskControl) return;
		
		loadingTaskControl = true;
		try {
			const response = await api.pauseScanning();
			if (response.data.success) {
				toast.success(response.data.message);
				await loadTaskControlStatus();
			} else {
				toast.error('暂停任务失败');
			}
		} catch (error) {
			console.error('暂停任务失败:', error);
			toast.error('暂停任务失败');
		} finally {
			loadingTaskControl = false;
		}
	}

	// 恢复所有任务
	async function resumeAllTasks() {
		if (loadingTaskControl) return;
		
		loadingTaskControl = true;
		try {
			const response = await api.resumeScanning();
			if (response.data.success) {
				toast.success(response.data.message);
				await loadTaskControlStatus();
			} else {
				toast.error('恢复任务失败');
			}
		} catch (error) {
			console.error('恢复任务失败:', error);
			toast.error('恢复任务失败');
		} finally {
			loadingTaskControl = false;
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

	// 从URL参数获取失败任务筛选状态
	function getShowFailedOnlyFromURL(searchParams: URLSearchParams): boolean {
		return searchParams.get('failed') === 'true';
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
		filter?: { type: string; id: string } | null,
		failedOnly: boolean = false
	) {
		loading = true;
		try {
			const params: Record<string, string | number | boolean> = {
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

			// 添加失败任务筛选参数
			if (failedOnly) {
				params.show_failed_only = true;
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
		const failedParam = showFailedOnly ? '&failed=true' : '';
		if (query) {
			goto(`/${query}&page=${pageNum}${failedParam}`);
		} else {
			goto(`/?page=${pageNum}${failedParam}`);
		}
	}

	async function handleSearchParamsChange() {
		const query = $page.url.searchParams.get('query');
		currentFilter = getFilterFromURL($page.url.searchParams);
		showFailedOnly = getShowFailedOnlyFromURL($page.url.searchParams);
		setQuery(query || '');
		if (currentFilter) {
			setVideoSourceFilter(currentFilter.type, currentFilter.id);
		} else {
			clearVideoSourceFilter();
		}
		loadVideos(query || '', parseInt($page.url.searchParams.get('page') || '0'), currentFilter, showFailedOnly);
	}

	function handleFilterRemove() {
		clearVideoSourceFilter();
		const failedParam = showFailedOnly ? '&failed=true' : '';
		goto(`/${ToQuery($appStateStore)}${failedParam}`);
	}

	// 切换失败任务筛选状态
	function toggleFailedTasksFilter() {
		showFailedOnly = !showFailedOnly;
		currentPage = 0; // 重置到第一页
		
		const query = ToQuery($appStateStore);
		const failedParam = showFailedOnly ? '&failed=true' : '';
		if (query) {
			goto(`/${query}${failedParam}`);
		} else {
			goto(`/${failedParam ? '?' + failedParam.slice(1) : ''}`);
		}
	}

	// 批量重置所有视频
	async function handleResetAllVideos() {
		resettingAll = true;
		try {
			let result;
			
			if (resetOptions.all) {
				// 重置所有失败任务，根据当前过滤器传递参数
				const filterParams = currentFilter ? {
					[currentFilter.type]: parseInt(currentFilter.id)
				} : undefined;
				result = await api.resetAllVideos(filterParams);
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
				
				// 调用选择性重置API，根据当前过滤器传递参数
				const filterParams = currentFilter ? {
					[currentFilter.type]: parseInt(currentFilter.id)
				} : undefined;
				result = await api.resetSpecificTasks(taskIndexes, filterParams);
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

	// 清理定时器
	onDestroy(() => {
		if (statusUpdateInterval) {
			clearInterval(statusUpdateInterval);
			statusUpdateInterval = null;
		}
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
	
	<!-- 失败任务筛选徽章 -->
	{#if showFailedOnly}
		<div class="mb-4">
			<div class="inline-flex items-center gap-2 rounded-full bg-red-100 px-3 py-1 text-sm text-red-800">
				<FilterIcon class="h-4 w-4" />
				<span>仅显示失败任务</span>
				<button
					onclick={toggleFailedTasksFilter}
					class="ml-1 hover:bg-red-200 rounded-full p-1"
					title="清除失败任务筛选"
					aria-label="清除失败任务筛选"
				>
					<svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
					</svg>
				</button>
			</div>
		</div>
	{/if}

	<!-- 统计信息和操作按钮 -->
	{#if videosData}
		<div class="mb-6 space-y-4 sm:space-y-0 sm:flex sm:items-center sm:justify-between">
			<!-- 统计信息 -->
			<div class="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4">
				<div class="text-muted-foreground text-sm">
					{#if showFailedOnly}
						共 {videosData.total_count} 个失败任务，{totalPages} 页
					{:else}
						共 {videosData.total_count} 个视频，{totalPages} 页
					{/if}
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
						onchange={(e) => {
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
				
				<!-- 任务控制按钮 -->
				{#if taskControlStatus}
					<Button
						size="sm"
						variant={taskControlStatus.is_paused ? "default" : "destructive"}
						onclick={taskControlStatus.is_paused ? resumeAllTasks : pauseAllTasks}
						disabled={loadingTaskControl}
						class="w-full sm:w-auto"
						title={taskControlStatus.is_paused ? '恢复所有下载和扫描任务' : '停止所有下载和扫描任务'}
					>
						{#if loadingTaskControl}
							<RotateCcwIcon class="mr-2 h-4 w-4 animate-spin" />
							处理中...
						{:else if taskControlStatus.is_paused}
							<PlayIcon class="mr-2 h-4 w-4" />
							恢复任务
						{:else}
							<PauseIcon class="mr-2 h-4 w-4" />
							停止任务
						{/if}
					</Button>
				{/if}
				
				<!-- 失败任务筛选按钮 -->
				<Button
					size="sm"
					variant={showFailedOnly ? "default" : "outline"}
					onclick={toggleFailedTasksFilter}
					class="w-full sm:w-auto"
					title={showFailedOnly ? '显示所有任务' : '仅显示失败任务'}
				>
					<FilterIcon class="mr-2 h-4 w-4" />
					{showFailedOnly ? '失败任务' : '显示失败'}
				</Button>

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
						onchange={(e) => handleResetOptionChange('all', e.currentTarget.checked)}
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
							onchange={(e) => handleResetOptionChange('videoCover', e.currentTarget.checked)}
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
							onchange={(e) => handleResetOptionChange('videoContent', e.currentTarget.checked)}
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
							onchange={(e) => handleResetOptionChange('videoInfo', e.currentTarget.checked)}
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
							onchange={(e) => handleResetOptionChange('videoDanmaku', e.currentTarget.checked)}
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
							onchange={(e) => handleResetOptionChange('videoSubtitle', e.currentTarget.checked)}
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
