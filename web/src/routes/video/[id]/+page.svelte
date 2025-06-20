<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import api from '$lib/api';
	import type { ApiError, VideoResponse, UpdateVideoStatusRequest } from '$lib/types';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
	import EditIcon from '@lucide/svelte/icons/edit';
	import PlayIcon from '@lucide/svelte/icons/play';
	import XIcon from '@lucide/svelte/icons/x';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { appStateStore, ToQuery } from '$lib/stores/filter';
	import VideoCard from '$lib/components/video-card.svelte';
	import StatusEditor from '$lib/components/status-editor.svelte';
	import { toast } from 'svelte-sonner';

	let videoData: VideoResponse | null = null;
	let loading = false;
	let error: string | null = null;
	let resetDialogOpen = false;
	let resetting = false;
	let statusEditorOpen = false;
	let statusEditorLoading = false;
	let showVideoPlayer = false;
	let currentPlayingPageIndex = 0;
	let onlinePlayMode = false; // false: 本地播放, true: 在线播放
	let onlinePlayInfo: any = null;
	let loadingPlayInfo = false;

	// 检查视频是否可播放（分P下载任务已完成）
	function isVideoPlayable(video: any): boolean {
		if (video && video.download_status && Array.isArray(video.download_status)) {
			// 检查第5个任务（分P下载，索引4）是否完成（状态为7）
			return video.download_status[4] === 7;
		}
		return false;
	}
	
	// 获取播放的视频ID（分页ID或视频ID）
	function getPlayVideoId(): number {
		if (videoData && videoData.pages && videoData.pages.length > 0) {
			// 如果有分页，使用分页ID
			return videoData.pages[currentPlayingPageIndex].id;
		} else if (videoData) {
			// 如果没有分页（单P视频），使用视频ID
			return videoData.video.id;
		}
		return 0;
	}

	async function loadVideoDetail() {
		const videoId = parseInt($page.params.id);
		if (isNaN(videoId)) {
			error = '无效的视频ID';
			toast.error('无效的视频ID');
			return;
		}

		loading = true;
		error = null;

		try {
			const result = await api.getVideo(videoId);
			videoData = result.data;
		} catch (error) {
			console.error('加载视频详情失败:', error);
			toast.error('加载视频详情失败', {
				description: (error as ApiError).message
			});
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		setBreadcrumb([
			{
				label: '主页',
				onClick: () => {
					goto(`/${ToQuery($appStateStore)}`);
				}
			},
			{ label: '视频详情', isActive: true }
		]);
	});

	// 监听路由参数变化
	$: if ($page.params.id) {
		loadVideoDetail();
	}

	async function handleStatusEditorSubmit(request: UpdateVideoStatusRequest) {
		if (!videoData) return;

		statusEditorLoading = true;
		try {
			const result = await api.updateVideoStatus(videoData.video.id, request);
			const data = result.data;

			if (data.success) {
				// 更新本地数据
				videoData = {
					video: data.video,
					pages: data.pages
				};
				statusEditorOpen = false;
				toast.success('状态更新成功');
			} else {
				toast.error('状态更新失败');
			}
		} catch (error) {
			console.error('状态更新失败:', error);
			toast.error('状态更新失败', {
				description: (error as ApiError).message
			});
		} finally {
			statusEditorLoading = false;
		}
	}

	// 获取在线播放信息
	async function loadOnlinePlayInfo(videoId: string | number) {
		if (loadingPlayInfo) return;
		
		loadingPlayInfo = true;
		try {
			const result = await api.getVideoPlayInfo(videoId);
			onlinePlayInfo = result.data;
			console.log('在线播放信息:', onlinePlayInfo);
		} catch (error) {
			console.error('获取播放信息失败:', error);
			toast.error('获取在线播放信息失败', {
				description: (error as ApiError).message
			});
			onlinePlayInfo = null;
		} finally {
			loadingPlayInfo = false;
		}
	}

	// 切换播放模式
	function togglePlayMode() {
		onlinePlayMode = !onlinePlayMode;
		if (onlinePlayMode && !onlinePlayInfo) {
			const videoId = getPlayVideoId();
			loadOnlinePlayInfo(videoId);
		}
	}

	// 获取视频播放源
	function getVideoSource() {
		if (onlinePlayMode && onlinePlayInfo) {
			// 在线播放模式：使用代理的B站视频流
			if (onlinePlayInfo.video_streams && onlinePlayInfo.video_streams.length > 0) {
				const videoStream = onlinePlayInfo.video_streams[0];
				return api.getProxyStreamUrl(videoStream.url);
			}
		} else {
			// 本地播放模式：使用现有的本地文件流
			return `/api/videos/stream/${getPlayVideoId()}`;
		}
		return '';
	}
</script>

<svelte:head>
	<title>{videoData?.video.name || '视频详情'} - Bili Sync</title>
</svelte:head>

{#if loading}
	<div class="flex items-center justify-center py-12">
		<div class="text-muted-foreground">加载中...</div>
	</div>
{:else if error}
	<div class="flex items-center justify-center py-12">
		<div class="space-y-2 text-center">
			<p class="text-destructive">{error}</p>
			<button
				class="text-muted-foreground hover:text-foreground text-sm transition-colors"
				onclick={() => goto('/')}
			>
				返回首页
			</button>
		</div>
	</div>
{:else if videoData}
	<!-- 视频信息区域 -->
	<section>
		<div class="mb-4 flex items-center justify-between">
			<h2 class="text-xl font-semibold">视频信息</h2>
			<div class="flex gap-2">
				{#if isVideoPlayable(videoData.video)}
					<Button
						size="sm"
						variant="default"
						class="shrink-0 cursor-pointer"
						onclick={() => (showVideoPlayer = true)}
					>
						<PlayIcon class="mr-2 h-4 w-4" />
						本地播放
					</Button>
				{/if}
				<Button
					size="sm"
					variant="outline"
					class="shrink-0 cursor-pointer"
					onclick={() => {
						onlinePlayMode = true;
						showVideoPlayer = true;
						if (!onlinePlayInfo) {
							const videoId = getPlayVideoId();
							loadOnlinePlayInfo(videoId);
						}
					}}
					disabled={loadingPlayInfo}
				>
					<PlayIcon class="mr-2 h-4 w-4" />
					{loadingPlayInfo ? '加载中...' : '在线播放'}
				</Button>
				<Button
					size="sm"
					variant="outline"
					class="shrink-0 cursor-pointer"
					onclick={() => (statusEditorOpen = true)}
					disabled={statusEditorLoading}
				>
					<EditIcon class="mr-2 h-4 w-4" />
					编辑状态
				</Button>
			</div>
		</div>

		<div style="margin-bottom: 1rem;">
			<VideoCard
				video={{
					id: videoData.video.id,
					name: videoData.video.name,
					upper_name: videoData.video.upper_name,
					path: videoData.video.path,
					category: videoData.video.category,
					download_status: videoData.video.download_status
				}}
				mode="detail"
				showActions={true}
				progressHeight="h-3"
				gap="gap-2"
				taskNames={['视频封面', '视频信息', 'UP主头像', 'UP主信息', '分P下载()']}
			/>
		</div>

		<!-- 下载路径信息 -->
		{#if videoData.pages && videoData.pages.length > 0 && videoData.pages[0].path}
			<div class="mb-4 rounded-lg border bg-gray-50 p-4">
				<h3 class="mb-2 text-sm font-medium text-gray-700">📁 下载保存路径</h3>
				<div class="rounded border bg-white px-3 py-2 font-mono text-sm break-all">
					{videoData.pages[0].path}
				</div>
				<p class="mt-1 text-xs text-gray-500">视频文件将保存到此路径下</p>
			</div>
		{/if}
	</section>

	<section>
		{#if videoData.pages && videoData.pages.length > 0}
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-xl font-semibold">分页列表</h2>
				<div class="text-muted-foreground text-sm">
					共 {videoData.pages.length} 个分页
				</div>
			</div>

			<!-- 响应式布局：大屏幕左右布局，小屏幕上下布局 -->
			<div class="flex flex-col xl:flex-row gap-6">
				<!-- 左侧/上方：分页列表 -->
				<div class="flex-1 min-w-0">
					<div
						class="grid gap-4"
						style="grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));"
					>
						{#each videoData.pages as pageInfo, index (pageInfo.id)}
							<div class="relative">
								<VideoCard
									video={{
										id: pageInfo.id,
										name: `P${pageInfo.pid}: ${pageInfo.name}`,
										upper_name: '',
										path: '',
										category: 0,
										download_status: pageInfo.download_status
									}}
									mode="page"
									showActions={false}
									customTitle="P{pageInfo.pid}: {pageInfo.name}"
									customSubtitle=""
									taskNames={['视频封面', '视频内容', '视频信息', '视频弹幕', '视频字幕']}
								/>
								<div class="absolute top-2 right-2 flex gap-1">
									{#if pageInfo.download_status[1] === 7}
										<Button
											size="sm"
											variant="ghost"
											class="h-8 w-8 p-0"
											title="本地播放"
											onclick={() => {
												currentPlayingPageIndex = index;
												onlinePlayMode = false;
												showVideoPlayer = true;
											}}
										>
											<PlayIcon class="h-4 w-4" />
										</Button>
									{/if}
									<Button
										size="sm"
										variant="ghost" 
										class="h-8 w-8 p-0"
										title="在线播放"
										onclick={() => {
											currentPlayingPageIndex = index;
											onlinePlayMode = true;
											showVideoPlayer = true;
											const videoId = getPlayVideoId();
											loadOnlinePlayInfo(videoId);
										}}
									>
										<PlayIcon class="h-3 w-3" />
										<span class="text-xs">在线</span>
									</Button>
								</div>
							</div>
						{/each}
					</div>
				</div>

				<!-- 右侧/下方：视频播放器 -->
				{#if showVideoPlayer && videoData}
					<div class="w-full xl:w-[45%] 2xl:w-[40%] shrink-0">
						<div class="sticky top-4">
							<div class="mb-4 flex items-center justify-between">
								<div class="flex items-center gap-2">
									<h3 class="text-lg font-semibold">视频播放</h3>
									<span class="text-sm px-2 py-1 rounded {onlinePlayMode ? 'bg-blue-100 text-blue-700' : 'bg-gray-100 text-gray-700'}">
										{onlinePlayMode ? '在线播放' : '本地播放'}
									</span>
									{#if onlinePlayMode && onlinePlayInfo}
										<span class="text-xs text-gray-500">
											{onlinePlayInfo.video_quality_description}
										</span>
									{/if}
								</div>
								<div class="flex items-center gap-2">
									<Button
										size="sm"
										variant="ghost"
										onclick={togglePlayMode}
										disabled={loadingPlayInfo}
									>
										{onlinePlayMode ? '切换到本地' : '切换到在线'}
									</Button>
									<Button
										size="sm"
										variant="outline"
										onclick={() => showVideoPlayer = false}
									>
										<XIcon class="mr-2 h-4 w-4" />
										关闭
									</Button>
								</div>
							</div>
							
							<!-- 当前播放的分页信息 -->
							{#if videoData.pages.length > 1}
								<div class="mb-2 text-sm text-gray-600">
									正在播放: P{videoData.pages[currentPlayingPageIndex].pid} - {videoData.pages[currentPlayingPageIndex].name}
								</div>
							{/if}
							
							<div class="bg-black rounded-lg overflow-hidden">
								{#if loadingPlayInfo && onlinePlayMode}
									<div class="flex items-center justify-center h-64 text-white">
										<div>加载播放信息中...</div>
									</div>
								{:else}
									{#key `${currentPlayingPageIndex}-${onlinePlayMode}`}
										<video 
											controls 
											autoplay
											class="w-full h-auto"
											style="aspect-ratio: 16/9; max-height: 70vh;"
											src={getVideoSource()}
											crossorigin="anonymous"
											onerror={(e) => {
												console.warn('视频加载错误:', e);
											}}
											onloadstart={() => {
												console.log('开始加载视频:', getVideoSource());
											}}
										>
											<!-- 默认空字幕轨道用于无障碍功能 -->
											<track kind="captions" srclang="zh" label="无字幕" default />
											{#if onlinePlayMode && onlinePlayInfo && onlinePlayInfo.subtitle_streams}
												{#each onlinePlayInfo.subtitle_streams as subtitle}
													<track 
														kind="subtitles" 
														srclang={subtitle.language}
														label={subtitle.language_doc}
														src={subtitle.url}
													/>
												{/each}
											{/if}
											您的浏览器不支持视频播放。
										</video>
									{/key}
								{/if}
							</div>
							
							<!-- 分页选择按钮 -->
							{#if videoData.pages.length > 1}
								<div class="mt-4 space-y-2">
									<div class="text-sm font-medium text-gray-700">选择分页:</div>
									<div class="grid grid-cols-2 gap-2 max-h-60 overflow-y-auto">
										{#each videoData.pages as page, index}
											{#if page.download_status[1] === 7}
												<Button
													size="sm"
													variant={currentPlayingPageIndex === index ? "default" : "outline"}
													class="text-left justify-start"
													onclick={() => {
														currentPlayingPageIndex = index;
														// 如果是在线播放模式，需要重新获取播放信息
														if (onlinePlayMode) {
															const videoId = getPlayVideoId();
															loadOnlinePlayInfo(videoId);
														} else {
															// 本地播放模式：强制重新加载视频
															setTimeout(() => {
																const videoElement = document.querySelector('video');
																if (videoElement) {
																	try {
																		videoElement.load();
																	} catch (e) {
																		console.warn('视频重载失败:', e);
																	}
																}
															}, 100);
														}
													}}
												>
													<span class="truncate">P{page.pid}: {page.name}</span>
												</Button>
											{/if}
										{/each}
									</div>
								</div>
							{/if}
						</div>
					</div>
				{/if}
			</div>
		{:else}
			<div class="py-12 text-center">
				<div class="space-y-2">
					<p class="text-muted-foreground">暂无分P数据</p>
					<p class="text-muted-foreground text-sm">该视频可能为单P视频</p>
				</div>
			</div>
		{/if}
	</section>

	<!-- 状态编辑器 -->
	{#if videoData}
		<StatusEditor
			bind:open={statusEditorOpen}
			video={videoData.video}
			pages={videoData.pages}
			loading={statusEditorLoading}
			onsubmit={handleStatusEditorSubmit}
		/>
	{/if}

{/if}
