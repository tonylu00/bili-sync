<script lang="ts">
	import VideoCard from '$lib/components/video-card.svelte';
	import FilterBadge from '$lib/components/filter-badge.svelte';
	import Pagination from '$lib/components/pagination.svelte';
	import AuthLogin from '$lib/components/auth-login.svelte';
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
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';

	// 认证状态
	let isAuthenticated = false;
	let videosData: VideosResponse | null = null;
	let loading = false;
	let currentPage = 0;
	const pageSize = 20;
	let currentFilter: { type: string; id: string } | null = null;
	let lastSearch: string | null = null;

	// 批量重置状态
	let resetAllDialogOpen = false;
	let resettingAll = false;

	// 处理登录成功
	function handleLoginSuccess(event: CustomEvent) {
		isAuthenticated = true;
		// 登录成功后加载数据
		loadInitialData();
	}

	// 退出登录
	function logout() {
		isAuthenticated = false;
		api.setAuthToken('');
		videosData = null;
		toast.info('已退出登录');
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
			const result = await api.resetAllVideos();
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

	$: if ($page.url.search !== lastSearch) {
		lastSearch = $page.url.search;
		handleSearchParamsChange();
	}

	// 检查认证状态
	onMount(async () => {
		const savedToken = localStorage.getItem('auth_token');
		if (savedToken && savedToken.trim()) {
			try {
				// 验证保存的 Token
				const sources = await api.getVideoSources();
				setVideoSources(sources.data);
				isAuthenticated = true;

				setBreadcrumb([
					{
						label: '主页',
						isActive: true
					}
				]);

				// 加载视频列表
				handleSearchParamsChange();
			} catch (error) {
				// Token 无效，清除
				api.setAuthToken('');
				console.error('Token 验证失败:', error);
			}
		}
	});

	$: totalPages = videosData ? Math.ceil(videosData.total_count / pageSize) : 0;
	$: filterTitle = currentFilter ? getFilterTitle(currentFilter.type) : '';
	$: filterName = currentFilter ? getFilterName(currentFilter.type, currentFilter.id) : '';
</script>

<svelte:head>
	<title>主页 - Bili Sync</title>
</svelte:head>

{#if !isAuthenticated}
	<AuthLogin on:login-success={handleLoginSuccess} />
{:else}
	<FilterBadge {filterTitle} {filterName} onRemove={handleFilterRemove} />

	<!-- 统计信息和操作按钮 -->
	{#if videosData}
		<div class="mb-6 flex items-center justify-between">
			<div class="text-muted-foreground text-sm">
				共 {videosData.total_count} 个视频，{totalPages} 页
			</div>
			<div class="flex gap-2">
				<Button
					size="sm"
					variant="outline"
					onclick={() => (resetAllDialogOpen = true)}
					disabled={resettingAll}
				>
					<RotateCcwIcon class="mr-2 h-4 w-4 {resettingAll ? 'animate-spin' : ''}" />
					批量重置
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
		<AlertDialog.Content>
			<AlertDialog.Header>
				<AlertDialog.Title>确认批量重置</AlertDialog.Title>
				<AlertDialog.Description>
					<p class="mb-2">确定要重置所有失败的视频下载状态吗？</p>
					<p class="text-muted-foreground text-sm">
						此操作会将所有失败状态的任务重置为未开始，无法撤销。
					</p>
				</AlertDialog.Description>
			</AlertDialog.Header>
			<AlertDialog.Footer>
				<AlertDialog.Cancel>取消</AlertDialog.Cancel>
				<AlertDialog.Action onclick={handleResetAllVideos} disabled={resettingAll}>
					{resettingAll ? '重置中...' : '确认重置'}
				</AlertDialog.Action>
			</AlertDialog.Footer>
		</AlertDialog.Content>
	</AlertDialog.Root>
{/if}
