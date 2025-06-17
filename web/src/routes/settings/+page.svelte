<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from '$lib/components/ui/sheet';
	import { toast } from 'svelte-sonner';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import api from '$lib/api';
	import { onMount } from 'svelte';
	import type { ConfigResponse, VideoInfo, ApiResponse, VideosResponse } from '$lib/types';
	import { TIMEZONE_OPTIONS, DEFAULT_TIMEZONE, getCurrentTimezone, setTimezone } from '$lib/utils/timezone';
	import { 
		FileTextIcon, 
		VideoIcon, 
		DownloadIcon, 
		MessageSquareIcon, 
		KeyIcon, 
		ShieldIcon, 
		SettingsIcon 
	} from 'lucide-svelte';

	let config: ConfigResponse | null = null;
	let loading = false;
	let saving = false;

	// 控制各个抽屉的开关状态
	let openSheet: string | null = null;
	
	// 随机视频封面背景
	let randomCovers: string[] = [];
	let currentBackgroundIndex = 0;
	
	// 获取代理后的图片URL
	function getProxiedImageUrl(originalUrl: string): string {
		if (!originalUrl) return '';
		// 使用后端代理端点
		return `/api/proxy/image?url=${encodeURIComponent(originalUrl)}`;
	}
	
	// 设置分类
	const settingCategories = [
		{
			id: 'naming',
			title: '文件命名',
			description: '配置视频、分页、番剧等文件命名模板',
			icon: FileTextIcon
		},
		{
			id: 'quality',
			title: '视频质量',
			description: '设置视频/音频质量、编解码器等参数',
			icon: VideoIcon
		},
		{
			id: 'download',
			title: '下载设置',
			description: '并行下载、并发控制、速率限制配置',
			icon: DownloadIcon
		},
		{
			id: 'danmaku',
			title: '弹幕设置',
			description: '弹幕显示样式和布局参数',
			icon: MessageSquareIcon
		},
		{
			id: 'credential',
			title: 'B站凭证',
			description: '配置B站登录凭证信息',
			icon: KeyIcon
		},
		{
			id: 'risk',
			title: '风控配置',
			description: 'UP主投稿获取风控策略',
			icon: ShieldIcon
		},
		{
			id: 'system',
			title: '系统设置',
			description: '时区、扫描间隔等其他设置',
			icon: SettingsIcon
		}
	];

	// 表单数据
	let videoName = '';
	let pageName = '';
	let multiPageName = '';
	let bangumiName = '';
	let folderStructure = '';
	let collectionFolderMode = 'separate';
	let timeFormat = '';
	let interval = 1200;
	let nfoTimeType = 'favtime';
	let parallelDownloadEnabled = false;
	let parallelDownloadThreads = 4;
	
	// 新增的配置数据
	let download_manager = 'httpx';
	let ffmpeg_path = '';
	let http_header: { [key: string]: string } = {};
	let download_rate_limit = 0;
	let multiple_parts_download = false;
	let use_proxy = false;
	let http_proxy = '';
	let credential = '';
	let cookies: { name: string; value: string; expires_at: number }[] = [];
	let global_path_filter: { type: string; value: string }[] = [];
	let headers: { [key: string]: string } = {};
	let webhooks = {
		video_refresh: { url: '', events: [] },
		video_download: { url: '', events: [] },
		other: { url: '', events: [] }
	};
	let min_free_space_gb = 10;
	let download_subtitle = true;
	let download_danmaku = true;
	let download_cover = true;
	let overwrite_mode = 'skip';
	let clear_temp_file = true;
	let mixed_download_mode = false;
	let disable_redirection = false;
	let watch_later_collection_name = 'biliwatch稍后再看';
	let enable_upload_notify = false;
	let enable_favorite_notify = false;
	let enable_https = false;
	let https_cert = '';
	let https_key = '';

	// 视频质量设置
	let videoMaxQuality = 'Quality8k';
	let videoMinQuality = 'Quality360p';
	let audioMaxQuality = 'QualityHiRES';
	let audioMinQuality = 'Quality64k';
	let codecs = ['AVC', 'HEV', 'AV1'];
	let noDolbyVideo = false;
	let noDolbyAudio = false;
	let noHdr = false;
	let noHires = false;

	// 弹幕设置
	let danmakuDuration = 15.0;
	let danmakuFont = '黑体';
	let danmakuFontSize = 25;
	let danmakuWidthRatio = 1.2;
	let danmakuHorizontalGap = 20.0;
	let danmakuLaneSize = 32;
	let danmakuFloatPercentage = 0.5;
	let danmakuBottomPercentage = 0.3;
	let danmakuOpacity = 76;
	let danmakuBold = true;
	let danmakuOutline = 0.8;
	let danmakuTimeOffset = 0.0;

	// 并发控制设置
	let concurrentVideo = 3;
	let concurrentPage = 2;
	let rateLimit = 4;
	let rateDuration = 250;

	// 其他设置
	let cdnSorting = false;
	let timezone = DEFAULT_TIMEZONE;

	// B站凭证设置
	let sessdata = '';
	let biliJct = '';
	let buvid3 = '';
	let dedeUserId = '';
	let acTimeValue = '';
	let credentialSaving = false;

	// UP主投稿风控配置
	let largeSubmissionThreshold = 100;
	let baseRequestDelay = 200;
	let largeSubmissionDelayMultiplier = 2;
	let enableProgressiveDelay = true;
	let maxDelayMultiplier = 4;
	let enableIncrementalFetch = true;
	let incrementalFallbackToFull = true;
	let enableBatchProcessing = false;
	let batchSize = 5;
	let batchDelaySeconds = 2;
	let enableAutoBackoff = true;
	let autoBackoffBaseSeconds = 10;
	let autoBackoffMaxMultiplier = 5;

	// 显示帮助信息的状态（在文件命名抽屉中使用）
	let showHelp = false;

	// 变量说明
	const variableHelp = {
		video: [
			{ name: '{{title}}', desc: '视频标题' },
			{ name: '{{bvid}}', desc: 'BV号（视频编号）' },
			{ name: '{{avid}}', desc: 'AV号（视频编号）' },
			{ name: '{{upper_name}}', desc: 'UP主名称' },
			{ name: '{{upper_mid}}', desc: 'UP主ID' },
			{ name: '{{pubtime}}', desc: '视频发布时间' },
			{ name: '{{fav_time}}', desc: '视频收藏时间（仅收藏夹视频有效）' }
		],
		page: [
			{ name: '{{ptitle}}', desc: '分页标题' },
			{ name: '{{pid}}', desc: '分页页号' },
			{ name: '{{pid_pad}}', desc: '补零的分页页号（如001、002）' },
			{ name: '{{season_pad}}', desc: '补零的季度号（多P视频默认为01）' }
		],
		common: [
			{ name: '{{ truncate title 10 }}', desc: '截取函数示例：截取标题前10个字符' },
			{ name: '路径分隔符', desc: '支持使用 / 或 \\\\ 创建子文件夹' }
		],
		time: [
			{ name: '%Y', desc: '年份（如2023）' },
			{ name: '%m', desc: '月份（如01-12）' },
			{ name: '%d', desc: '日期（如01-31）' },
			{ name: '%H', desc: '小时（如00-23）' },
			{ name: '%M', desc: '分钟（如00-59）' },
			{ name: '%S', desc: '秒数（如00-59）' }
		]
	};

	// NFO 时间类型选项
	const nfoTimeTypeOptions = [
		{ value: 'favtime', label: '收藏时间' },
		{ value: 'pubtime', label: '发布时间' }
	];

	// 视频质量选项
	const videoQualityOptions = [
		{ value: 'Quality8k', label: '8K超高清' },
		{ value: 'Quality4k', label: '4K超高清' },
		{ value: 'Quality1080pplus', label: '1080P+高码率' },
		{ value: 'Quality1080p60', label: '1080P 60fps' },
		{ value: 'Quality1080p', label: '1080P高清' },
		{ value: 'Quality720p60', label: '720P 60fps' },
		{ value: 'Quality720p', label: '720P高清' },
		{ value: 'Quality480p', label: '480P清晰' },
		{ value: 'Quality360p', label: '360P流畅' }
	];

	// 音频质量选项
	const audioQualityOptions = [
		{ value: 'QualityHiRES', label: 'Hi-Res无损' },
		{ value: 'Quality320k', label: '320k高品质' },
		{ value: 'Quality128k', label: '128k标准' },
		{ value: 'Quality64k', label: '64k省流' }
	];

	// 编解码器选项
	const codecOptions = [
		{ value: 'AVC', label: 'AVC/H.264' },
		{ value: 'HEV', label: 'HEVC/H.265' },
		{ value: 'AV1', label: 'AV1' }
	];

	// 响应式相关
	let innerWidth: number;
	let isMobile: boolean = false;
	$: isMobile = innerWidth < 768; // md断点

	// 拖拽排序相关
	let draggedIndex: number | null = null;

	function handleDragStart(e: DragEvent, index: number) {
		if (e.dataTransfer) {
			draggedIndex = index;
			e.dataTransfer.effectAllowed = 'move';
			e.dataTransfer.setData('text/html', '');
		}
	}

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer) {
			e.dataTransfer.dropEffect = 'move';
		}
	}

	function handleDrop(e: DragEvent, dropIndex: number) {
		e.preventDefault();
		if (draggedIndex !== null && draggedIndex !== dropIndex) {
			const newCodecs = [...codecs];
			const draggedItem = newCodecs[draggedIndex];
			newCodecs.splice(draggedIndex, 1);
			newCodecs.splice(dropIndex, 0, draggedItem);
			codecs = newCodecs;
		}
		draggedIndex = null;
	}

	function removeCodec(index: number) {
		codecs = codecs.filter((_, i) => i !== index);
	}

	function handleAddCodec(e: Event) {
		const target = e.target as HTMLSelectElement;
		const value = target.value;
		if (value && !codecs.includes(value)) {
			codecs = [...codecs, value];
			target.value = '';
		}
	}

	onMount(async () => {
		setBreadcrumb([
			{ label: '主页', href: '/' },
			{ label: '设置', isActive: true }
		]);

		await loadConfig();
		await loadRandomCovers();
	});
	
	async function loadRandomCovers() {
		try {
			// 获取一些随机视频封面
			const response = await api.getVideos({ page_size: 20 });
			if (response.data && response.data.videos) {
				// 提取封面URL并过滤掉无效的，同时转换为代理URL
				randomCovers = response.data.videos
					.filter((video: VideoInfo) => video.cover && video.cover.length > 0)
					.map((video: VideoInfo) => getProxiedImageUrl(video.cover));
			}
		} catch (error) {
			console.error('Failed to load random covers:', error);
		}
	}
	
	// 当打开抽屉时切换背景
	$: if (openSheet && randomCovers.length > 0) {
		currentBackgroundIndex = Math.floor(Math.random() * randomCovers.length);
		console.log('Current background:', randomCovers[currentBackgroundIndex]);
	}

	async function loadConfig() {
		loading = true;
		try {
			const response = await api.getConfig();
			config = response.data;

			// 填充表单
			videoName = config.video_name || '';
			pageName = config.page_name || '';
			multiPageName = config.multi_page_name || '';
			bangumiName = config.bangumi_name || '';
			folderStructure = config.folder_structure || '';
			collectionFolderMode = config.collection_folder_mode || 'separate';
			timeFormat = config.time_format || '';
			interval = config.interval || 1200;
			nfoTimeType = config.nfo_time_type || 'favtime';
			parallelDownloadEnabled = config.parallel_download_enabled || false;
			parallelDownloadThreads = config.parallel_download_threads || 4;

			// 视频质量设置
			videoMaxQuality = config.video_max_quality || 'Quality8k';
			videoMinQuality = config.video_min_quality || 'Quality360p';
			audioMaxQuality = config.audio_max_quality || 'QualityHiRES';
			audioMinQuality = config.audio_min_quality || 'Quality64k';
			codecs = config.codecs || ['AVC', 'HEV', 'AV1'];
			noDolbyVideo = config.no_dolby_video || false;
			noDolbyAudio = config.no_dolby_audio || false;
			noHdr = config.no_hdr || false;
			noHires = config.no_hires || false;

			// 弹幕设置
			danmakuDuration = config.danmaku_duration || 15.0;
			danmakuFont = config.danmaku_font || '黑体';
			danmakuFontSize = config.danmaku_font_size || 25;
			danmakuWidthRatio = config.danmaku_width_ratio || 1.2;
			danmakuHorizontalGap = config.danmaku_horizontal_gap || 20.0;
			danmakuLaneSize = config.danmaku_lane_size || 32;
			danmakuFloatPercentage = config.danmaku_float_percentage || 0.5;
			danmakuBottomPercentage = config.danmaku_bottom_percentage || 0.3;
			danmakuOpacity = config.danmaku_opacity || 76;
			danmakuBold = config.danmaku_bold !== undefined ? config.danmaku_bold : true;
			danmakuOutline = config.danmaku_outline || 0.8;
			danmakuTimeOffset = config.danmaku_time_offset || 0.0;

			// 并发控制设置
			concurrentVideo = config.concurrent_video || 3;
			concurrentPage = config.concurrent_page || 2;
			rateLimit = config.rate_limit || 4;
			rateDuration = config.rate_duration || 250;

			// 其他设置
			cdnSorting = config.cdn_sorting || false;
			timezone = config.timezone || getCurrentTimezone();

			// B站凭证设置
			sessdata = config.credential?.sessdata || '';
			biliJct = config.credential?.bili_jct || '';
			buvid3 = config.credential?.buvid3 || '';
			dedeUserId = config.credential?.dedeuserid || '';
			acTimeValue = config.credential?.ac_time_value || '';

			// UP主投稿风控配置
			largeSubmissionThreshold = config.large_submission_threshold || 100;
			baseRequestDelay = config.base_request_delay || 200;
			largeSubmissionDelayMultiplier = config.large_submission_delay_multiplier || 2;
			enableProgressiveDelay = config.enable_progressive_delay || true;
			maxDelayMultiplier = config.max_delay_multiplier || 4;
			enableIncrementalFetch = config.enable_incremental_fetch || true;
			incrementalFallbackToFull = config.incremental_fallback_to_full || true;
			enableBatchProcessing = config.enable_batch_processing || false;
			batchSize = config.batch_size || 5;
			batchDelaySeconds = config.batch_delay_seconds || 2;
			enableAutoBackoff = config.enable_auto_backoff || true;
			autoBackoffBaseSeconds = config.auto_backoff_base_seconds || 10;
			autoBackoffMaxMultiplier = config.auto_backoff_max_multiplier || 5;
			
			// 加载新增的配置数据
			download_manager = config.download_manager || 'httpx';
			ffmpeg_path = config.ffmpeg_path || '';
			http_header = config.http_header || {};
			download_rate_limit = config.download_rate_limit || 0;
			multiple_parts_download = config.multiple_parts_download || false;
			use_proxy = config.use_proxy || false;
			http_proxy = config.http_proxy || '';
			credential = config.credential || '';
			cookies = config.cookies || [];
			global_path_filter = config.global_path_filter || [];
			headers = config.headers || {};
			webhooks = config.webhooks || {
				video_refresh: { url: '', events: [] },
				video_download: { url: '', events: [] },
				other: { url: '', events: [] }
			};
			min_free_space_gb = config.min_free_space_gb || 10;
			download_subtitle = config.download_subtitle || true;
			download_danmaku = config.download_danmaku || true;
			download_cover = config.download_cover || true;
			overwrite_mode = config.overwrite_mode || 'skip';
			clear_temp_file = config.clear_temp_file || true;
			mixed_download_mode = config.mixed_download_mode || false;
			disable_redirection = config.disable_redirection || false;
			watch_later_collection_name = config.watch_later_collection_name || 'biliwatch稍后再看';
			enable_upload_notify = config.enable_upload_notify || false;
			enable_favorite_notify = config.enable_favorite_notify || false;
			enable_https = config.enable_https || false;
			https_cert = config.https_cert || '';
			https_key = config.https_key || '';
		} catch (error: any) {
			console.error('加载配置失败:', error);
			toast.error('加载配置失败', { description: error.message });
		} finally {
			loading = false;
		}
	}

	async function saveConfig() {
		saving = true;
		try {
			const params = {
				video_name: videoName,
				page_name: pageName,
				multi_page_name: multiPageName,
				bangumi_name: bangumiName,
				folder_structure: folderStructure,
				collection_folder_mode: collectionFolderMode,
				time_format: timeFormat,
				interval: interval,
				nfo_time_type: nfoTimeType,
				parallel_download_enabled: parallelDownloadEnabled,
				parallel_download_threads: parallelDownloadThreads,
				// 视频质量设置
				video_max_quality: videoMaxQuality,
				video_min_quality: videoMinQuality,
				audio_max_quality: audioMaxQuality,
				audio_min_quality: audioMinQuality,
				codecs: codecs,
				no_dolby_video: noDolbyVideo,
				no_dolby_audio: noDolbyAudio,
				no_hdr: noHdr,
				no_hires: noHires,
				// 弹幕设置
				danmaku_duration: danmakuDuration,
				danmaku_font: danmakuFont,
				danmaku_font_size: danmakuFontSize,
				danmaku_width_ratio: danmakuWidthRatio,
				danmaku_horizontal_gap: danmakuHorizontalGap,
				danmaku_lane_size: danmakuLaneSize,
				danmaku_float_percentage: danmakuFloatPercentage,
				danmaku_bottom_percentage: danmakuBottomPercentage,
				danmaku_opacity: danmakuOpacity,
				danmaku_bold: danmakuBold,
				danmaku_outline: danmakuOutline,
				danmaku_time_offset: danmakuTimeOffset,
				// 并发控制设置
				concurrent_video: concurrentVideo,
				concurrent_page: concurrentPage,
				rate_limit: rateLimit,
				rate_duration: rateDuration,
				// 其他设置
				cdn_sorting: cdnSorting,
				timezone: timezone,
				// UP主投稿风控配置
				large_submission_threshold: largeSubmissionThreshold,
				base_request_delay: baseRequestDelay,
				large_submission_delay_multiplier: largeSubmissionDelayMultiplier,
				enable_progressive_delay: enableProgressiveDelay,
				max_delay_multiplier: maxDelayMultiplier,
				enable_incremental_fetch: enableIncrementalFetch,
				incremental_fallback_to_full: incrementalFallbackToFull,
				enable_batch_processing: enableBatchProcessing,
				batch_size: batchSize,
				batch_delay_seconds: batchDelaySeconds,
				enable_auto_backoff: enableAutoBackoff,
				auto_backoff_base_seconds: autoBackoffBaseSeconds,
				auto_backoff_max_multiplier: autoBackoffMaxMultiplier,
				// 新增的配置数据
				download_manager: download_manager,
				ffmpeg_path: ffmpeg_path,
				http_header: http_header,
				download_rate_limit: download_rate_limit,
				multiple_parts_download: multiple_parts_download,
				use_proxy: use_proxy,
				http_proxy: http_proxy,
				credential: credential,
				cookies: cookies,
				global_path_filter: global_path_filter,
				headers: headers,
				webhooks: webhooks,
				min_free_space_gb: min_free_space_gb,
				download_subtitle: download_subtitle,
				download_danmaku: download_danmaku,
				download_cover: download_cover,
				overwrite_mode: overwrite_mode,
				clear_temp_file: clear_temp_file,
				mixed_download_mode: mixed_download_mode,
				disable_redirection: disable_redirection,
				watch_later_collection_name: watch_later_collection_name,
				enable_upload_notify: enable_upload_notify,
				enable_favorite_notify: enable_favorite_notify,
				enable_https: enable_https,
				https_cert: https_cert,
				https_key: https_key
			};

			const response = await api.updateConfig(params);

			if (response.data.success) {
				toast.success('保存成功', { description: response.data.message });
				openSheet = null; // 关闭抽屉
			} else {
				toast.error('保存失败', { description: response.data.message });
			}
		} catch (error: any) {
			console.error('保存配置失败:', error);
			toast.error('保存失败', { description: error.message });
		} finally {
			saving = false;
		}
	}

	async function saveCredential() {
		credentialSaving = true;
		try {
			const params = {
				sessdata: sessdata.trim(),
				bili_jct: biliJct.trim(),
				buvid3: buvid3.trim(),
				dedeuserid: dedeUserId.trim(),
				ac_time_value: acTimeValue.trim()
			};

			const response = await api.updateCredential(params);

			if (response.data.success) {
				toast.success('B站凭证保存成功', { description: response.data.message });
				// 重新加载配置以获取最新状态
				await loadConfig();
				openSheet = null; // 关闭抽屉
			} else {
				toast.error('保存失败', { description: response.data.message });
			}
		} catch (error: any) {
			console.error('保存B站凭证失败:', error);
			toast.error('保存失败', { description: error.message });
		} finally {
			credentialSaving = false;
		}
	}
</script>

<svelte:head>
	<title>设置 - Bili Sync</title>
</svelte:head>

<svelte:window bind:innerWidth />

<div class="py-2">
	<div class="mx-auto px-4">
		<div class="bg-card rounded-lg border p-6 shadow-sm">
			<h1 class="mb-6 text-2xl font-bold">系统设置</h1>

			{#if loading}
				<div class="flex items-center justify-center py-12">
					<div class="text-muted-foreground">加载中...</div>
				</div>
			{:else}
				<!-- 设置分类卡片列表 -->
				<div class="grid gap-4 {isMobile ? 'grid-cols-1' : 'grid-cols-2 lg:grid-cols-3'}">
					{#each settingCategories as category}
						<Card 
							class="cursor-pointer transition-all hover:shadow-md hover:border-primary/50"
							onclick={() => openSheet = category.id}
						>
							<CardHeader>
								<div class="flex items-start gap-3">
									<div class="p-2 rounded-lg bg-primary/10">
										<svelte:component this={category.icon} class="h-5 w-5 text-primary" />
									</div>
									<div class="flex-1">
										<CardTitle class="text-base">{category.title}</CardTitle>
										<CardDescription class="text-sm mt-1">{category.description}</CardDescription>
									</div>
								</div>
							</CardHeader>
						</Card>
					{/each}
				</div>
			{/if}
		</div>
	</div>
</div>

<!-- 文件命名设置抽屉 -->
<Sheet open={openSheet === 'naming'} onOpenChange={(open) => !open && (openSheet = null)}>
	<SheetContent side={isMobile ? 'bottom' : 'right'} class="{isMobile ? 'h-[85vh] max-h-[85vh]' : '!w-screen !h-screen !max-w-none !inset-y-0 !right-0'} {!isMobile ? 'overflow-hidden' : ''}">
		{#if !isMobile && randomCovers.length > 0}
			<!-- 电脑端背景图 -->
			<div class="absolute inset-0" style="z-index: 0;">
				<img 
					src={randomCovers[currentBackgroundIndex]} 
					alt="背景"
					class="w-full h-full object-cover"
					style="opacity: 0.6; filter: contrast(1.1) brightness(0.9);"
					loading="lazy"
					onerror={(e) => console.error('Image load error:', e)}
				/>
				<div class="absolute inset-0" style="background: linear-gradient(to bottom right, rgba(255,255,255,0.85), rgba(255,255,255,0.5));"></div>
			</div>
		{/if}
		<div class="h-full flex items-center justify-center {isMobile ? '' : 'p-8'} relative" style="z-index: 1;">
			<div class="{isMobile ? 'w-full h-full bg-background' : 'max-w-4xl w-full bg-white/90 backdrop-blur-md rounded-lg shadow-2xl border'} overflow-hidden relative">
				<SheetHeader class="{isMobile ? '' : 'p-6 border-b'}">
					<SheetTitle>文件命名设置</SheetTitle>
					<SheetDescription>配置视频、分页、番剧等文件命名模板</SheetDescription>
				</SheetHeader>
				<form onsubmit={(e) => { e.preventDefault(); saveConfig(); }} class="flex flex-col {isMobile ? 'h-[calc(100%-5rem)]' : 'h-[calc(100%-8rem)]'}">
					<div class="flex-1 overflow-y-auto px-6 py-6 space-y-6">
						<div class="flex items-center justify-between">
							<h3 class="text-base font-semibold">文件命名模板</h3>
							<button
								type="button"
								onclick={() => showHelp = !showHelp}
								class="text-sm text-blue-600 hover:text-blue-800"
							>
								{showHelp ? '隐藏' : '显示'}变量说明
							</button>
						</div>

						{#if showHelp}
							<div class="rounded-lg border border-blue-200 bg-blue-50 p-4">
								<div class="grid grid-cols-1 gap-4 text-sm md:grid-cols-2">
									<div>
										<h4 class="mb-2 font-medium text-blue-900">视频变量</h4>
										<div class="space-y-1">
											{#each variableHelp.video as item}
												<div class="flex">
													<code class="mr-2 rounded bg-blue-100 px-1 text-blue-800">{item.name}</code>
													<span class="text-gray-600">{item.desc}</span>
												</div>
											{/each}
										</div>
									</div>
									<div>
										<h4 class="mb-2 font-medium text-blue-900">分页变量</h4>
										<div class="space-y-1">
											{#each variableHelp.page as item}
												<div class="flex">
													<code class="mr-2 rounded bg-blue-100 px-1 text-blue-800">{item.name}</code>
													<span class="text-gray-600">{item.desc}</span>
												</div>
											{/each}
										</div>
										<h4 class="mb-2 mt-4 font-medium text-blue-900">通用函数</h4>
										<div class="space-y-1">
											{#each variableHelp.common as item}
												<div class="flex">
													<code class="mr-2 rounded bg-blue-100 px-1 text-blue-800">{item.name}</code>
													<span class="text-gray-600">{item.desc}</span>
												</div>
											{/each}
										</div>
									</div>
									<div class="md:col-span-2">
										<h4 class="mb-2 font-medium text-blue-900">时间格式变量</h4>
										<div class="grid grid-cols-3 gap-2">
											{#each variableHelp.time as item}
												<div class="flex">
													<code class="mr-2 rounded bg-blue-100 px-1 text-blue-800">{item.name}</code>
													<span class="text-gray-600">{item.desc}</span>
												</div>
											{/each}
										</div>
									</div>
								</div>
							</div>
						{/if}

						<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
							<div class="space-y-2">
								<Label for="video-name">视频文件名模板</Label>
								<Input
									id="video-name"
									bind:value={videoName}
									placeholder="{{title}}"
								/>
							</div>

							<div class="space-y-2">
								<Label for="page-name">单P视频文件名模板</Label>
								<Input
									id="page-name"
									bind:value={pageName}
									placeholder="{{bvid}}"
								/>
							</div>

							<div class="space-y-2">
								<Label for="multi-page-name">多P视频文件名模板</Label>
								<Input
									id="multi-page-name"
									bind:value={multiPageName}
									placeholder="{{bvid}}/{{bvid}}.P{{pid_pad}}.{{ptitle}}"
								/>
							</div>

							<div class="space-y-2">
								<Label for="bangumi-name">番剧文件名模板</Label>
								<Input
									id="bangumi-name"
									bind:value={bangumiName}
									placeholder="{{title}}/Season {{season_pad}}/{{title}} - S{{season_pad}}E{{pid_pad}}"
								/>
							</div>
						</div>

						<div class="space-y-2">
							<Label for="folder-structure">文件夹结构模板</Label>
							<Input
								id="folder-structure"
								bind:value={folderStructure}
								placeholder="{{upper_name}}/{{title}}"
							/>
							<p class="text-muted-foreground text-sm">
								定义视频文件的文件夹层级结构
							</p>
						</div>

						<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
							<div class="space-y-2">
								<Label for="collection-folder-mode">合集文件夹模式</Label>
								<select
									id="collection-folder-mode"
									bind:value={collectionFolderMode}
									class="border-input bg-background ring-offset-background focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
								>
									<option value="separate">分离模式</option>
									<option value="nested">嵌套模式</option>
								</select>
								<p class="text-muted-foreground text-sm">
									分离：合集名作为单独文件夹<br />
									嵌套：合集名嵌入文件夹结构
								</p>
							</div>

							<div class="space-y-2">
								<Label for="time-format">时间格式</Label>
								<Input
									id="time-format"
									bind:value={timeFormat}
									placeholder="%Y-%m-%d %H-%M-%S"
								/>
								<p class="text-muted-foreground text-sm">
									控制时间变量的显示格式
								</p>
							</div>
						</div>

						<div class="space-y-2">
							<Label for="nfo-time-type">NFO文件时间类型</Label>
							<select
								id="nfo-time-type"
								bind:value={nfoTimeType}
								class="border-input bg-background ring-offset-background focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
							>
								{#each nfoTimeTypeOptions as option}
									<option value={option.value}>{option.label}</option>
								{/each}
							</select>
							<p class="text-muted-foreground text-sm">
								选择NFO文件中使用的时间类型
							</p>
						</div>

						<div class="rounded-lg border border-orange-200 bg-orange-50 p-3">
							<h5 class="mb-2 font-medium text-orange-800">命名模板说明</h5>
							<div class="space-y-1 text-orange-700 text-sm">
								<p>• 使用双花括号 {{}} 包裹变量名</p>
								<p>• 支持使用 / 或 \\ 创建子文件夹</p>
								<p>• 非法字符会自动替换为下划线</p>
								<p>• 时间变量需要配合时间格式使用</p>
							</div>
						</div>
					</div>
					<SheetFooter class="border-t pt-4 pb-safe">
						<Button type="submit" disabled={saving} class="w-full">
							{saving ? '保存中...' : '保存设置'}
						</Button>
					</SheetFooter>
				</form>
			</div>
		</div>
	</SheetContent>
</Sheet>

<!-- 视频质量设置抽屉 -->
<Sheet open={openSheet === 'quality'} onOpenChange={(open) => !open && (openSheet = null)}>
	<SheetContent side={isMobile ? 'bottom' : 'right'} class={isMobile ? 'h-[85vh] max-h-[85vh]' : '!w-screen !h-screen !max-w-none !inset-y-0 !right-0'}>
		{#if !isMobile && randomCovers.length > 0}
			<!-- 电脑端背景图 -->
			<div class="absolute inset-0 z-0 overflow-hidden">
				<img 
					src={randomCovers[(currentBackgroundIndex + 1) % randomCovers.length]} 
					alt="背景"
					class="w-full h-full object-cover"
					style="opacity: 0.6; filter: contrast(1.1) brightness(0.9);"
					loading="lazy"
				/>
				<div class="absolute inset-0" style="background: linear-gradient(to bottom right, rgba(255,255,255,0.85), rgba(255,255,255,0.5));"></div>
			</div>
		{/if}
		<div class="h-full flex items-center justify-center {isMobile ? '' : 'p-8'} relative z-10">
			<div class="{isMobile ? 'w-full h-full bg-background' : 'max-w-4xl w-full bg-card/95 backdrop-blur-sm rounded-lg shadow-2xl border'} overflow-hidden relative">
				<SheetHeader class="{isMobile ? '' : 'p-6 border-b'}">
					<SheetTitle>视频质量设置</SheetTitle>
					<SheetDescription>设置视频/音频质量、编解码器等参数</SheetDescription>
				</SheetHeader>
				<form onsubmit={(e) => { e.preventDefault(); saveConfig(); }} class="flex flex-col {isMobile ? 'h-[calc(100%-5rem)]' : 'h-[calc(100%-8rem)]'}">
					<div class="flex-1 overflow-y-auto px-6 py-6 space-y-6">
				<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
					<div class="space-y-2">
						<Label for="video-max-quality">视频最高质量</Label>
						<select
							id="video-max-quality"
							bind:value={videoMaxQuality}
							class="border-input bg-background ring-offset-background focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
						>
							{#each videoQualityOptions as option}
								<option value={option.value}>{option.label}</option>
							{/each}
						</select>
					</div>

					<div class="space-y-2">
						<Label for="video-min-quality">视频最低质量</Label>
						<select
							id="video-min-quality"
							bind:value={videoMinQuality}
							class="border-input bg-background ring-offset-background focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
						>
							{#each videoQualityOptions as option}
								<option value={option.value}>{option.label}</option>
							{/each}
						</select>
					</div>

					<div class="space-y-2">
						<Label for="audio-max-quality">音频最高质量</Label>
						<select
							id="audio-max-quality"
							bind:value={audioMaxQuality}
							class="border-input bg-background ring-offset-background focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
						>
							{#each audioQualityOptions as option}
								<option value={option.value}>{option.label}</option>
							{/each}
						</select>
					</div>

					<div class="space-y-2">
						<Label for="audio-min-quality">音频最低质量</Label>
						<select
							id="audio-min-quality"
							bind:value={audioMinQuality}
							class="border-input bg-background ring-offset-background focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
						>
							{#each audioQualityOptions as option}
								<option value={option.value}>{option.label}</option>
							{/each}
						</select>
					</div>
				</div>

				<div class="space-y-2">
					<Label>编解码器优先级顺序</Label>
					<p class="text-muted-foreground mb-3 text-sm">
						拖拽以调整优先级，越靠前优先级越高。根据设备硬件解码支持情况选择：
					</p>
					<div class="mb-3 rounded-lg border border-blue-200 bg-blue-50 p-3">
						<div class="space-y-2 text-xs text-blue-700">
							<div>
								<strong>🎯 AVC (H.264)：</strong>兼容性最好，几乎所有设备都支持硬件解码，播放流畅，但文件体积较大
							</div>
							<div>
								<strong>🚀 HEV (H.265)：</strong>新一代编码，体积更小，需要较新设备硬件解码支持
							</div>
							<div>
								<strong>⚡ AV1：</strong>最新编码格式，压缩率最高，需要最新设备支持，软解可能卡顿
							</div>
							<div class="mt-2 border-t border-blue-300 pt-1">
								<strong>💡 推荐设置：</strong>如果设备较老或追求兼容性，将AVC放首位；如果设备支持新编码且网络较慢，可优先HEV或AV1
							</div>
						</div>
					</div>
					<div class="space-y-2">
						{#each codecs as codec, index}
							<div
								class="flex cursor-move items-center gap-3 rounded-lg border bg-gray-50 p-3"
								draggable="true"
								ondragstart={(e) => handleDragStart(e, index)}
								ondragover={handleDragOver}
								ondrop={(e) => handleDrop(e, index)}
								role="button"
								tabindex="0"
							>
								<div class="flex items-center gap-2 text-gray-400">
									<svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
										<path d="M7 2a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2H7zM8 6h4v2H8V6zm0 4h4v2H8v-2z" />
									</svg>
								</div>
								<div class="flex flex-1 items-center gap-2">
									<span class="bg-primary text-primary-foreground flex h-6 w-6 items-center justify-center rounded-full text-sm font-medium">
										{index + 1}
									</span>
									<span class="font-medium">
										{codecOptions.find((option) => option.value === codec)?.label || codec}
									</span>
								</div>
								<button
									type="button"
									class="p-1 text-red-500 hover:text-red-700"
									onclick={() => removeCodec(index)}
									title="移除此编解码器"
									aria-label="移除此编解码器"
								>
									<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
									</svg>
								</button>
							</div>
						{/each}

						{#if codecs.length < codecOptions.length}
							<div class="mt-2">
								<select
									class="w-full rounded-md border p-2 text-sm"
									onchange={handleAddCodec}
									value=""
								>
									<option value="" disabled>添加编解码器...</option>
									{#each codecOptions as option}
										{#if !codecs.includes(option.value)}
											<option value={option.value}>{option.label}</option>
										{/if}
									{/each}
								</select>
							</div>
						{/if}
					</div>
				</div>

				<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
					<div class="flex items-center space-x-2">
						<input
							type="checkbox"
							id="no-dolby-video"
							bind:checked={noDolbyVideo}
							class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
						/>
						<Label for="no-dolby-video" class="text-sm">禁用杜比视界</Label>
					</div>

					<div class="flex items-center space-x-2">
						<input
							type="checkbox"
							id="no-dolby-audio"
							bind:checked={noDolbyAudio}
							class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
						/>
						<Label for="no-dolby-audio" class="text-sm">禁用杜比全景声</Label>
					</div>

					<div class="flex items-center space-x-2">
						<input
							type="checkbox"
							id="no-hdr"
							bind:checked={noHdr}
							class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
						/>
						<Label for="no-hdr" class="text-sm">禁用HDR</Label>
					</div>

					<div class="flex items-center space-x-2">
						<input
							type="checkbox"
							id="no-hires"
							bind:checked={noHires}
							class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
						/>
						<Label for="no-hires" class="text-sm">禁用Hi-Res音频</Label>
					</div>
				</div>
					</div>
					<SheetFooter class="border-t pt-4 pb-safe">
						<Button type="submit" disabled={saving} class="w-full">
							{saving ? '保存中...' : '保存设置'}
						</Button>
					</SheetFooter>
				</form>
			</div>
		</div>
	</SheetContent>
</Sheet>

<!-- 下载设置抽屉 -->
<Sheet open={openSheet === 'download'} onOpenChange={(open) => !open && (openSheet = null)}>
	<SheetContent side={isMobile ? 'bottom' : 'right'} class={isMobile ? 'h-[85vh] max-h-[85vh]' : '!w-screen !h-screen !max-w-none !inset-y-0 !right-0'}>
		{#if !isMobile && randomCovers.length > 0}
			<!-- 电脑端背景图 -->
			<div class="absolute inset-0 z-0 overflow-hidden">
				<img 
					src={randomCovers[(currentBackgroundIndex + 2) % randomCovers.length]} 
					alt="背景"
					class="w-full h-full object-cover"
					style="opacity: 0.6; filter: contrast(1.1) brightness(0.9);"
					loading="lazy"
				/>
				<div class="absolute inset-0" style="background: linear-gradient(to bottom right, rgba(255,255,255,0.85), rgba(255,255,255,0.5));"></div>
			</div>
		{/if}
		<div class="h-full flex items-center justify-center {isMobile ? '' : 'p-8'} relative z-10">
			<div class="{isMobile ? 'w-full h-full bg-background' : 'max-w-4xl w-full bg-card/95 backdrop-blur-sm rounded-lg shadow-2xl border'} overflow-hidden relative">
				<SheetHeader class="{isMobile ? '' : 'p-6 border-b'}">
					<SheetTitle>下载设置</SheetTitle>
					<SheetDescription>并行下载、并发控制、速率限制配置</SheetDescription>
				</SheetHeader>
				<form onsubmit={(e) => { e.preventDefault(); saveConfig(); }} class="flex flex-col {isMobile ? 'h-[calc(100%-5rem)]' : 'h-[calc(100%-8rem)]'}">
					<div class="flex-1 overflow-y-auto px-6 py-6 space-y-6">
						
						<div class="mt-6 space-y-6">
							<h3 class="text-base font-semibold">下载配置</h3>
							
							<div class="flex items-center space-x-2">
								<input
									type="checkbox"
									id="parallel-download"
									bind:checked={parallelDownloadEnabled}
									class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
								/>
								<Label for="parallel-download" class="text-sm leading-none font-medium peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
									启用多线程下载
								</Label>
							</div>

							{#if parallelDownloadEnabled}
								<div class="ml-6 space-y-2">
									<Label for="threads">下载线程数</Label>
									<Input
										id="threads"
										type="number"
										bind:value={parallelDownloadThreads}
										min="1"
										max="16"
										placeholder="4"
									/>
								</div>
							{/if}
						</div>

						<div class="mt-6 space-y-6">
							<h3 class="text-base font-semibold">并发控制</h3>
							
							<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
								<div class="space-y-2">
									<Label for="concurrent-video">同时处理视频数</Label>
									<Input
										id="concurrent-video"
										type="number"
										bind:value={concurrentVideo}
										min="1"
										max="10"
										placeholder="3"
									/>
								</div>

								<div class="space-y-2">
									<Label for="concurrent-page">每个视频并发分页数</Label>
									<Input
										id="concurrent-page"
										type="number"
										bind:value={concurrentPage}
										min="1"
										max="10"
										placeholder="2"
									/>
								</div>

								<div class="space-y-2">
									<Label for="rate-limit">请求频率限制</Label>
									<Input
										id="rate-limit"
										type="number"
										bind:value={rateLimit}
										min="1"
										max="100"
										placeholder="4"
									/>
									<p class="text-muted-foreground text-sm">每个时间窗口内的最大请求数</p>
								</div>

								<div class="space-y-2">
									<Label for="rate-duration">时间窗口（毫秒）</Label>
									<Input
										id="rate-duration"
										type="number"
										bind:value={rateDuration}
										min="100"
										max="5000"
										placeholder="250"
									/>
									<p class="text-muted-foreground text-sm">请求频率限制的时间窗口</p>
								</div>
							</div>
						</div>

						<div class="mt-6 rounded-lg border border-purple-200 bg-purple-50 p-3">
							<h5 class="mb-2 font-medium text-purple-800">并发控制说明</h5>
							<div class="space-y-1 text-purple-700 text-sm">
								<p><strong>视频并发数：</strong>同时处理的视频数量（建议1-5）</p>
								<p><strong>分页并发数：</strong>每个视频内的并发分页数（建议1-3）</p>
								<p><strong>请求频率限制：</strong>防止API请求过频繁导致风控，调小limit可减少被限制</p>
								<p><strong>总并行度：</strong>约等于 视频并发数 × 分页并发数</p>
							</div>
						</div>
					</div>
					<SheetFooter class="border-t pt-4 pb-safe">
						<Button type="submit" disabled={saving} class="w-full">
							{saving ? '保存中...' : '保存设置'}
						</Button>
					</SheetFooter>
				</form>
			</div>
		</div>
	</SheetContent>
</Sheet>

<!-- 弹幕设置抽屉 -->
<Sheet open={openSheet === 'danmaku'} onOpenChange={(open) => !open && (openSheet = null)}>
	<SheetContent side={isMobile ? 'bottom' : 'right'} class={isMobile ? 'h-[85vh] max-h-[85vh]' : '!w-screen !h-screen !max-w-none !inset-y-0 !right-0'}>
		{#if !isMobile && randomCovers.length > 0}
			<!-- 电脑端背景图 -->
			<div class="absolute inset-0 z-0 overflow-hidden">
				<img 
					src={randomCovers[(currentBackgroundIndex + 3) % randomCovers.length]} 
					alt="背景"
					class="w-full h-full object-cover"
					style="opacity: 0.6; filter: contrast(1.1) brightness(0.9);"
					loading="lazy"
				/>
				<div class="absolute inset-0" style="background: linear-gradient(to bottom right, rgba(255,255,255,0.85), rgba(255,255,255,0.5));"></div>
			</div>
		{/if}
		<div class="h-full flex items-center justify-center {isMobile ? '' : 'p-8'} relative z-10">
			<div class="{isMobile ? 'w-full h-full bg-background' : 'max-w-4xl w-full bg-card/95 backdrop-blur-sm rounded-lg shadow-2xl border'} overflow-hidden relative">
				<SheetHeader class="{isMobile ? '' : 'p-6 border-b'}">
					<SheetTitle>弹幕设置</SheetTitle>
					<SheetDescription>弹幕显示样式和布局参数</SheetDescription>
				</SheetHeader>
				<form onsubmit={(e) => { e.preventDefault(); saveConfig(); }} class="flex flex-col {isMobile ? 'h-[calc(100%-5rem)]' : 'h-[calc(100%-8rem)]'}">
					<div class="flex-1 overflow-y-auto px-6 py-6 space-y-6">
				<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
					<div class="space-y-2">
						<Label for="danmaku-duration">弹幕持续时间（秒）</Label>
						<Input
							id="danmaku-duration"
							type="number"
							bind:value={danmakuDuration}
							min="1"
							max="60"
							step="0.1"
							placeholder="15.0"
						/>
					</div>

					<div class="space-y-2">
						<Label for="danmaku-font">字体</Label>
						<Input id="danmaku-font" bind:value={danmakuFont} placeholder="黑体" />
					</div>

					<div class="space-y-2">
						<Label for="danmaku-font-size">字体大小</Label>
						<Input
							id="danmaku-font-size"
							type="number"
							bind:value={danmakuFontSize}
							min="10"
							max="100"
							placeholder="25"
						/>
					</div>

					<div class="space-y-2">
						<Label for="danmaku-width-ratio">宽度比例</Label>
						<Input
							id="danmaku-width-ratio"
							type="number"
							bind:value={danmakuWidthRatio}
							min="0.1"
							max="3.0"
							step="0.1"
							placeholder="1.2"
						/>
					</div>

					<div class="space-y-2">
						<Label for="danmaku-horizontal-gap">水平间距</Label>
						<Input
							id="danmaku-horizontal-gap"
							type="number"
							bind:value={danmakuHorizontalGap}
							min="0"
							max="100"
							step="1"
							placeholder="20.0"
						/>
					</div>

					<div class="space-y-2">
						<Label for="danmaku-lane-size">轨道高度</Label>
						<Input
							id="danmaku-lane-size"
							type="number"
							bind:value={danmakuLaneSize}
							min="10"
							max="100"
							placeholder="32"
						/>
					</div>

					<div class="space-y-2">
						<Label for="danmaku-float-percentage">滚动弹幕占比</Label>
						<Input
							id="danmaku-float-percentage"
							type="number"
							bind:value={danmakuFloatPercentage}
							min="0"
							max="1"
							step="0.1"
							placeholder="0.5"
						/>
					</div>

					<div class="space-y-2">
						<Label for="danmaku-bottom-percentage">底部弹幕占比</Label>
						<Input
							id="danmaku-bottom-percentage"
							type="number"
							bind:value={danmakuBottomPercentage}
							min="0"
							max="1"
							step="0.1"
							placeholder="0.3"
						/>
					</div>

					<div class="space-y-2">
						<Label for="danmaku-opacity">不透明度</Label>
						<Input
							id="danmaku-opacity"
							type="number"
							bind:value={danmakuOpacity}
							min="0"
							max="100"
							placeholder="76"
						/>
					</div>

					<div class="space-y-2">
						<Label for="danmaku-outline">描边宽度</Label>
						<Input
							id="danmaku-outline"
							type="number"
							bind:value={danmakuOutline}
							min="0"
							max="5"
							step="0.1"
							placeholder="0.8"
						/>
					</div>

					<div class="space-y-2">
						<Label for="danmaku-time-offset">时间偏移（秒）</Label>
						<Input
							id="danmaku-time-offset"
							type="number"
							bind:value={danmakuTimeOffset}
							step="0.1"
							placeholder="0.0"
						/>
					</div>

					<div class="flex items-center space-x-2">
						<input
							type="checkbox"
							id="danmaku-bold"
							bind:checked={danmakuBold}
							class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
						/>
						<Label for="danmaku-bold" class="text-sm">加粗字体</Label>
					</div>
				</div>

				<div class="rounded-lg border border-green-200 bg-green-50 p-3">
					<h5 class="mb-2 font-medium text-green-800">弹幕设置说明</h5>
					<div class="space-y-1 text-green-700 text-sm">
						<p><strong>持续时间：</strong>弹幕在屏幕上显示的时间（秒）</p>
						<p><strong>字体样式：</strong>字体、大小、加粗、描边等外观设置</p>
						<p><strong>布局设置：</strong>轨道高度、间距、占比等位置控制</p>
						<p><strong>时间偏移：</strong>正值延后弹幕，负值提前弹幕</p>
					</div>
				</div>
					</div>
					<SheetFooter class="border-t pt-4 pb-safe">
						<Button type="submit" disabled={saving} class="w-full">
							{saving ? '保存中...' : '保存设置'}
						</Button>
					</SheetFooter>
				</form>
			</div>
		</div>
	</SheetContent>
</Sheet>

<!-- B站凭证设置抽屉 -->
<Sheet open={openSheet === 'credential'} onOpenChange={(open) => !open && (openSheet = null)}>
	<SheetContent side={isMobile ? 'bottom' : 'right'} class={isMobile ? 'h-[85vh] max-h-[85vh]' : '!w-screen !h-screen !max-w-none !inset-y-0 !right-0'}>
		{#if !isMobile && randomCovers.length > 0}
			<!-- 电脑端背景图 -->
			<div class="absolute inset-0 z-0 overflow-hidden">
				<img 
					src={randomCovers[(currentBackgroundIndex + 4) % randomCovers.length]} 
					alt="背景"
					class="w-full h-full object-cover"
					style="opacity: 0.6; filter: contrast(1.1) brightness(0.9);"
					loading="lazy"
				/>
				<div class="absolute inset-0" style="background: linear-gradient(to bottom right, rgba(255,255,255,0.85), rgba(255,255,255,0.5));"></div>
			</div>
		{/if}
		<div class="h-full flex items-center justify-center {isMobile ? '' : 'p-8'} relative z-10">
			<div class="{isMobile ? 'w-full h-full bg-background' : 'max-w-4xl w-full bg-card/95 backdrop-blur-sm rounded-lg shadow-2xl border'} overflow-hidden relative">
				<SheetHeader class="{isMobile ? '' : 'p-6 border-b'}">
					<SheetTitle>B站凭证设置</SheetTitle>
					<SheetDescription>配置B站登录凭证信息</SheetDescription>
				</SheetHeader>
				<form onsubmit={(e) => { e.preventDefault(); saveCredential(); }} class="flex flex-col {isMobile ? 'h-[calc(100%-5rem)]' : 'h-[calc(100%-8rem)]'}">
					<div class="flex-1 overflow-y-auto px-6 py-6 space-y-6">
				<div class="rounded-lg border border-amber-200 bg-amber-50 p-4">
					<div class="space-y-2 text-sm text-amber-800">
						<div class="font-medium">🔐 如何获取B站登录凭证：</div>
						<ol class="ml-4 list-decimal space-y-1">
							<li>在浏览器中登录B站</li>
							<li>按F12打开开发者工具</li>
							<li>切换到"网络"(Network)标签</li>
							<li>刷新页面，找到任意一个请求</li>
							<li>在请求头中找到Cookie字段，复制对应的值</li>
						</ol>
						<div class="mt-2 text-xs text-amber-600">
							💡 提示：SESSDATA、bili_jct、buvid3、DedeUserID是必填项，ac_time_value可选
						</div>
					</div>
				</div>

				<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
					<div class="space-y-2">
						<Label for="sessdata">SESSDATA *</Label>
						<Input
							id="sessdata"
							type="password"
							bind:value={sessdata}
							placeholder="请输入SESSDATA"
						/>
					</div>

					<div class="space-y-2">
						<Label for="bili-jct">bili_jct *</Label>
						<Input
							id="bili-jct"
							type="password"
							bind:value={biliJct}
							placeholder="请输入bili_jct"
						/>
					</div>

					<div class="space-y-2">
						<Label for="buvid3">buvid3 *</Label>
						<Input
							id="buvid3"
							bind:value={buvid3}
							placeholder="请输入buvid3"
						/>
					</div>

					<div class="space-y-2">
						<Label for="dedeuserid">DedeUserID *</Label>
						<Input
							id="dedeuserid"
							bind:value={dedeUserId}
							placeholder="请输入DedeUserID"
						/>
					</div>

					<div class="space-y-2 md:col-span-2">
						<Label for="ac-time-value">ac_time_value (可选)</Label>
						<Input
							id="ac-time-value"
							bind:value={acTimeValue}
							placeholder="请输入ac_time_value（可选）"
						/>
					</div>
				</div>

				<div class="rounded-lg border border-green-200 bg-green-50 p-3">
					<div class="text-sm text-green-800">
						<div class="font-medium mb-1">✅ 凭证状态检查：</div>
						<div class="text-xs">
							{#if sessdata && biliJct && buvid3 && dedeUserId}
								<span class="text-green-600">✓ 必填凭证已填写完整</span>
							{:else}
								<span class="text-orange-600">⚠ 请填写所有必填凭证项</span>
							{/if}
						</div>
					</div>
				</div>
					</div>
					<SheetFooter class="border-t pt-4 pb-safe">
						<Button type="submit" disabled={credentialSaving} class="w-full">
							{credentialSaving ? '保存中...' : '保存凭证'}
						</Button>
					</SheetFooter>
				</form>
			</div>
		</div>
	</SheetContent>
</Sheet>

<!-- 风控配置抽屉 -->
<Sheet open={openSheet === 'risk'} onOpenChange={(open) => !open && (openSheet = null)}>
	<SheetContent side={isMobile ? 'bottom' : 'right'} class={isMobile ? 'h-[85vh] max-h-[85vh]' : '!w-screen !h-screen !max-w-none !inset-y-0 !right-0'}>
		{#if !isMobile && randomCovers.length > 0}
			<!-- 电脑端背景图 -->
			<div class="absolute inset-0 z-0 overflow-hidden">
				<img 
					src={randomCovers[(currentBackgroundIndex + 5) % randomCovers.length]} 
					alt="背景"
					class="w-full h-full object-cover"
					style="opacity: 0.6; filter: contrast(1.1) brightness(0.9);"
					loading="lazy"
				/>
				<div class="absolute inset-0" style="background: linear-gradient(to bottom right, rgba(255,255,255,0.85), rgba(255,255,255,0.5));"></div>
			</div>
		{/if}
		<div class="h-full flex items-center justify-center {isMobile ? '' : 'p-8'} relative z-10">
			<div class="{isMobile ? 'w-full h-full bg-background' : 'max-w-4xl w-full bg-card/95 backdrop-blur-sm rounded-lg shadow-2xl border'} overflow-hidden relative">
				<SheetHeader class="{isMobile ? '' : 'p-6 border-b'}">
					<SheetTitle>风控配置</SheetTitle>
					<SheetDescription>UP主投稿获取风控策略，用于优化大量视频UP主的获取</SheetDescription>
				</SheetHeader>
				<form onsubmit={(e) => { e.preventDefault(); saveConfig(); }} class="flex flex-col {isMobile ? 'h-[calc(100%-5rem)]' : 'h-[calc(100%-8rem)]'}">
					<div class="flex-1 overflow-y-auto px-6 py-6 space-y-6">
				<!-- 基础优化配置 -->
				<div class="rounded-lg border border-blue-200 bg-blue-50 p-4">
					<h3 class="mb-3 text-sm font-medium text-blue-800">🎯 基础优化配置</h3>
					<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
						<div class="space-y-2">
							<Label for="large-submission-threshold">大量视频UP主阈值</Label>
							<Input
								id="large-submission-threshold"
								type="number"
								bind:value={largeSubmissionThreshold}
								min="10"
								max="1000"
								placeholder="100"
							/>
							<p class="text-muted-foreground text-xs">超过此视频数量的UP主将启用风控策略</p>
						</div>

						<div class="space-y-2">
							<Label for="base-request-delay">基础请求间隔（毫秒）</Label>
							<Input
								id="base-request-delay"
								type="number"
								bind:value={baseRequestDelay}
								min="50"
								max="2000"
								placeholder="200"
							/>
							<p class="text-muted-foreground text-xs">每个请求之间的基础延迟时间</p>
						</div>

						<div class="space-y-2">
							<Label for="large-submission-delay-multiplier">大量视频延迟倍数</Label>
							<Input
								id="large-submission-delay-multiplier"
								type="number"
								bind:value={largeSubmissionDelayMultiplier}
								min="1"
								max="10"
								step="0.5"
								placeholder="2"
							/>
							<p class="text-muted-foreground text-xs">大量视频UP主的延迟倍数</p>
						</div>

						<div class="space-y-2">
							<Label for="max-delay-multiplier">最大延迟倍数</Label>
							<Input
								id="max-delay-multiplier"
								type="number"
								bind:value={maxDelayMultiplier}
								min="1"
								max="20"
								step="0.5"
								placeholder="4"
							/>
							<p class="text-muted-foreground text-xs">渐进式延迟的最大倍数限制</p>
						</div>
					</div>

					<div class="mt-4 flex items-center space-x-2">
						<input
							type="checkbox"
							id="enable-progressive-delay"
							bind:checked={enableProgressiveDelay}
							class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
						/>
						<Label for="enable-progressive-delay" class="text-sm">启用渐进式延迟</Label>
						<p class="text-muted-foreground ml-2 text-xs">随着请求次数增加逐步延长延迟时间</p>
					</div>
				</div>

				<!-- 增量获取配置 -->
				<div class="rounded-lg border border-green-200 bg-green-50 p-4">
					<h3 class="mb-3 text-sm font-medium text-green-800">📈 增量获取配置</h3>
					<div class="space-y-4">
						<div class="flex items-center space-x-2">
							<input
								type="checkbox"
								id="enable-incremental-fetch"
								bind:checked={enableIncrementalFetch}
								class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
							/>
							<Label for="enable-incremental-fetch" class="text-sm">启用增量获取</Label>
							<p class="text-muted-foreground ml-2 text-xs">优先获取最新视频，减少不必要的请求</p>
						</div>

						<div class="flex items-center space-x-2">
							<input
								type="checkbox"
								id="incremental-fallback-to-full"
								bind:checked={incrementalFallbackToFull}
								class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
							/>
							<Label for="incremental-fallback-to-full" class="text-sm">增量获取失败时回退到全量获取</Label>
							<p class="text-muted-foreground ml-2 text-xs">确保数据完整性</p>
						</div>
					</div>
				</div>

				<!-- 分批处理配置 -->
				<div class="rounded-lg border border-purple-200 bg-purple-50 p-4">
					<h3 class="mb-3 text-sm font-medium text-purple-800">📦 分批处理配置</h3>
					<div class="space-y-4">
						<div class="flex items-center space-x-2">
							<input
								type="checkbox"
								id="enable-batch-processing"
								bind:checked={enableBatchProcessing}
								class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
							/>
							<Label for="enable-batch-processing" class="text-sm">启用分批处理</Label>
							<p class="text-muted-foreground ml-2 text-xs">将大量请求分批处理，降低服务器压力</p>
						</div>

						{#if enableBatchProcessing}
							<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
								<div class="space-y-2">
									<Label for="batch-size">分批大小（页数）</Label>
									<Input
										id="batch-size"
										type="number"
										bind:value={batchSize}
										min="1"
										max="20"
										placeholder="5"
									/>
									<p class="text-muted-foreground text-xs">每批处理的页数</p>
								</div>

								<div class="space-y-2">
									<Label for="batch-delay-seconds">批次间延迟（秒）</Label>
									<Input
										id="batch-delay-seconds"
										type="number"
										bind:value={batchDelaySeconds}
										min="1"
										max="60"
										placeholder="2"
									/>
									<p class="text-muted-foreground text-xs">每批之间的等待时间</p>
								</div>
							</div>
						{/if}
					</div>
				</div>

				<!-- 自动退避配置 -->
				<div class="rounded-lg border border-orange-200 bg-orange-50 p-4">
					<h3 class="mb-3 text-sm font-medium text-orange-800">🔄 自动退避配置</h3>
					<div class="space-y-4">
						<div class="flex items-center space-x-2">
							<input
								type="checkbox"
								id="enable-auto-backoff"
								bind:checked={enableAutoBackoff}
								class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
							/>
							<Label for="enable-auto-backoff" class="text-sm">启用自动退避</Label>
							<p class="text-muted-foreground ml-2 text-xs">遇到错误时自动增加延迟时间</p>
						</div>

						{#if enableAutoBackoff}
							<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
								<div class="space-y-2">
									<Label for="auto-backoff-base-seconds">自动退避基础时间（秒）</Label>
									<Input
										id="auto-backoff-base-seconds"
										type="number"
										bind:value={autoBackoffBaseSeconds}
										min="1"
										max="300"
										placeholder="10"
									/>
									<p class="text-muted-foreground text-xs">遇到错误时的基础等待时间</p>
								</div>

								<div class="space-y-2">
									<Label for="auto-backoff-max-multiplier">自动退避最大倍数</Label>
									<Input
										id="auto-backoff-max-multiplier"
										type="number"
										bind:value={autoBackoffMaxMultiplier}
										min="1"
										max="20"
										placeholder="5"
									/>
									<p class="text-muted-foreground text-xs">退避时间的最大倍数限制</p>
								</div>
							</div>
						{/if}
					</div>
				</div>

				<!-- 使用建议 -->
				<div class="rounded-lg border border-gray-200 bg-gray-50 p-4">
					<h3 class="mb-3 text-sm font-medium text-gray-800">💡 使用建议</h3>
					<div class="space-y-2 text-xs text-gray-600">
						<p><strong>小型UP主（&lt;100视频）：</strong> 使用默认设置即可</p>
						<p><strong>中型UP主（100-500视频）：</strong> 启用渐进式延迟和增量获取</p>
						<p><strong>大型UP主（500-1000视频）：</strong> 启用分批处理，设置较大的延迟倍数</p>
						<p><strong>超大型UP主（&gt;1000视频）：</strong> 启用所有风控策略，适当增加各项延迟参数</p>
						<p><strong>频繁遇到412错误：</strong> 增加基础请求间隔和延迟倍数</p>
					</div>
				</div>
					</div>
					<SheetFooter class="border-t pt-4 pb-safe">
						<Button type="submit" disabled={saving} class="w-full">
							{saving ? '保存中...' : '保存设置'}
						</Button>
					</SheetFooter>
				</form>
			</div>
		</div>
	</SheetContent>
</Sheet>

<!-- 系统设置抽屉 -->
<Sheet open={openSheet === 'system'} onOpenChange={(open) => !open && (openSheet = null)}>
	<SheetContent side={isMobile ? 'bottom' : 'right'} class={isMobile ? 'h-[85vh] max-h-[85vh]' : '!w-screen !h-screen !max-w-none !inset-y-0 !right-0'}>
		{#if !isMobile && randomCovers.length > 0}
			<!-- 电脑端背景图 -->
			<div class="absolute inset-0 z-0 overflow-hidden">
				<img 
					src={randomCovers[(currentBackgroundIndex + 6) % randomCovers.length]} 
					alt="背景"
					class="w-full h-full object-cover"
					style="opacity: 0.6; filter: contrast(1.1) brightness(0.9);"
					loading="lazy"
				/>
				<div class="absolute inset-0" style="background: linear-gradient(to bottom right, rgba(255,255,255,0.85), rgba(255,255,255,0.5));"></div>
			</div>
		{/if}
		<div class="h-full flex items-center justify-center {isMobile ? '' : 'p-8'} relative z-10">
			<div class="{isMobile ? 'w-full h-full bg-background' : 'max-w-4xl w-full bg-card/95 backdrop-blur-sm rounded-lg shadow-2xl border'} overflow-hidden relative">
				<SheetHeader class="{isMobile ? '' : 'p-6 border-b'}">
					<SheetTitle>系统设置</SheetTitle>
					<SheetDescription>时区、扫描间隔等其他设置</SheetDescription>
				</SheetHeader>
				<form onsubmit={(e) => { e.preventDefault(); saveConfig(); }} class="flex flex-col {isMobile ? 'h-[calc(100%-5rem)]' : 'h-[calc(100%-8rem)]'}">
					<div class="flex-1 overflow-y-auto px-6 py-6 space-y-6">
						
						<!-- Basic System Settings -->
						<div class="mt-6 space-y-6">
							<h3 class="text-base font-semibold">基本系统设置</h3>
							
							<div class="space-y-2">
								<Label for="interval">扫描间隔（秒）</Label>
								<Input
									id="interval"
									type="number"
									bind:value={interval}
									min="60"
									placeholder="1200"
								/>
								<p class="text-muted-foreground text-sm">每次扫描下载的时间间隔</p>
							</div>

							<div class="space-y-2">
								<Label for="timezone">时区设置</Label>
								<select
									id="timezone"
									bind:value={timezone}
									onchange={() => setTimezone(timezone)}
									class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
								>
									{#each TIMEZONE_OPTIONS as option}
										<option value={option.value}>{option.label}</option>
									{/each}
								</select>
								<p class="text-muted-foreground text-sm">
									选择时区后，所有时间戳将转换为对应时区显示
								</p>
							</div>

							<div class="flex items-center space-x-2">
								<input
									type="checkbox"
									id="cdn-sorting"
									bind:checked={cdnSorting}
									class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
								/>
								<Label for="cdn-sorting" class="text-sm">启用CDN排序</Label>
								<p class="text-muted-foreground ml-2 text-sm">优化下载节点选择</p>
							</div>

							<div class="rounded-lg border border-orange-200 bg-orange-50 p-3">
								<h5 class="mb-2 font-medium text-orange-800">其他设置说明</h5>
								<div class="space-y-1 text-orange-700 text-sm">
									<p><strong>扫描间隔：</strong>每次扫描下载的时间间隔（秒）</p>
									<p><strong>时间格式：</strong>控制时间变量在文件名中的显示格式</p>
									<p><strong>CDN排序：</strong>启用后优先使用质量更高的CDN，可能提升下载速度</p>
								</div>
							</div>
						</div>
					</div>
					<SheetFooter class="border-t pt-4 pb-safe">
						<Button type="submit" disabled={saving} class="w-full">
							{saving ? '保存中...' : '保存设置'}
						</Button>
					</SheetFooter>
				</form>
			</div>
		</div>
	</SheetContent>
</Sheet>