<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Button } from '$lib/components/ui/button';
	import { toast } from 'svelte-sonner';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import api from '$lib/api';
	import { onMount } from 'svelte';
	import type { ConfigResponse } from '$lib/types';

	let config: ConfigResponse | null = null;
	let loading = false;
	let saving = false;

	// 表单数据
	let videoName = '';
	let pageName = '';
	let multiPageName = '';
	let bangumiName = '';
	let folderStructure = '';
	let timeFormat = '';
	let interval = 1200;
	let nfoTimeType = 'favtime';
	let parallelDownloadEnabled = false;
	let parallelDownloadThreads = 4;

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

	// 显示帮助信息的状态
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
	});

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
				cdn_sorting: cdnSorting
			};

			const response = await api.updateConfig(params);

			if (response.data.success) {
				toast.success('保存成功', { description: response.data.message });
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
				<div class="flex {isMobile ? 'flex-col' : 'gap-8'}">
					<!-- 左侧：表单区域 -->
					<div class={isMobile ? 'w-full' : 'w-[600px] flex-shrink-0'}>
						<form
							onsubmit={(e) => {
								e.preventDefault();
								saveConfig();
							}}
							class="space-y-8"
						>
							<!-- 文件命名模板 -->
							<div class="space-y-6">
								<div class="flex {isMobile ? 'flex-col gap-2' : 'items-center justify-between'}">
									<h2 class="text-lg font-semibold">文件命名模板</h2>
									<Button
										type="button"
										variant="outline"
										size="sm"
										onclick={() => (showHelp = !showHelp)}
										class={isMobile ? 'w-full' : ''}
									>
										{showHelp ? '隐藏' : '显示'}变量说明
									</Button>
								</div>

								<div class="space-y-2">
									<Label for="video-name">视频文件名</Label>
									<Input id="video-name" bind:value={videoName} placeholder={'{{title}}'} />
									<p class="text-muted-foreground text-sm">
										可用变量：&#123;&#123;title&#125;&#125;, &#123;&#123;bvid&#125;&#125;,
										&#123;&#123;avid&#125;&#125;
									</p>
								</div>

								<div class="space-y-2">
									<Label for="page-name">分P视频名</Label>
									<Input id="page-name" bind:value={pageName} placeholder={'{{title}}'} />
									<p class="text-muted-foreground text-sm">单P视频的命名模板</p>
								</div>

								<div class="space-y-2">
									<Label for="multi-page-name">多P视频名</Label>
									<Input
										id="multi-page-name"
										bind:value={multiPageName}
										placeholder={'{{title}}-P{{pid_pad}}'}
									/>
									<p class="text-muted-foreground text-sm">
										多P视频必须包含分页标识符，如 &#123;&#123;pid&#125;&#125; 或
										&#123;&#123;pid_pad&#125;&#125;
									</p>
								</div>

								<div class="space-y-2">
									<Label for="bangumi-name">番剧文件名</Label>
									<Input
										id="bangumi-name"
										bind:value={bangumiName}
										placeholder={'S{{season_pad}}E{{pid_pad}}-{{pid_pad}}'}
									/>
									<p class="text-muted-foreground text-sm">番剧专用模板，必须包含分页标识符</p>
								</div>

								<div class="space-y-2">
									<Label for="folder-structure">文件夹结构</Label>
									<Input
										id="folder-structure"
										bind:value={folderStructure}
										placeholder="Season 1"
									/>
								</div>
							</div>

							<!-- 系统设置 -->
							<div class="space-y-6">
								<h2 class="text-lg font-semibold">系统设置</h2>

								<div class="space-y-2">
									<Label for="time-format">时间格式</Label>
									<Input id="time-format" bind:value={timeFormat} placeholder="%Y-%m-%d" />
									<p class="text-muted-foreground text-sm">Python strftime 格式</p>
								</div>

								<div class="space-y-2">
									<Label for="interval">扫描间隔（秒）</Label>
									<Input
										id="interval"
										type="number"
										bind:value={interval}
										min="60"
										placeholder="1200"
									/>
								</div>

								<div class="space-y-2">
									<Label for="nfo-time-type">NFO 时间类型</Label>
									<select
										id="nfo-time-type"
										bind:value={nfoTimeType}
										class="border-input bg-background ring-offset-background focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
									>
										{#each nfoTimeTypeOptions as option}
											<option value={option.value}>{option.label}</option>
										{/each}
									</select>
								</div>
							</div>

							<!-- 下载设置 -->
							<div class="space-y-6">
								<h2 class="text-lg font-semibold">下载设置</h2>

								<div class="flex items-center space-x-2">
									<input
										type="checkbox"
										id="parallel-download"
										bind:checked={parallelDownloadEnabled}
										class="text-primary focus:ring-primary h-4 w-4 rounded border-gray-300"
									/>
									<Label
										for="parallel-download"
										class="text-sm leading-none font-medium peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
									>
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

							<!-- 视频质量设置 -->
							<div class="space-y-6">
								<h2 class="text-lg font-semibold">视频质量设置</h2>

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
												<strong>🎯 AVC (H.264)：</strong
												>兼容性最好，几乎所有设备都支持硬件解码，播放流畅，但文件体积较大
											</div>
											<div>
												<strong>🚀 HEV (H.265)：</strong
												>新一代编码，体积更小，需要较新设备硬件解码支持
											</div>
											<div>
												<strong>⚡ AV1：</strong
												>最新编码格式，压缩率最高，需要最新设备支持，软解可能卡顿
											</div>
											<div class="mt-2 border-t border-blue-300 pt-1">
												<strong>💡 推荐设置：</strong
												>如果设备较老或追求兼容性，将AVC放首位；如果设备支持新编码且网络较慢，可优先HEV或AV1
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
														<path
															d="M7 2a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2H7zM8 6h4v2H8V6zm0 4h4v2H8v-2z"
														/>
													</svg>
												</div>
												<div class="flex flex-1 items-center gap-2">
													<span
														class="bg-primary text-primary-foreground flex h-6 w-6 items-center justify-center rounded-full text-sm font-medium"
													>
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
													<svg
														class="h-4 w-4"
														fill="none"
														stroke="currentColor"
														viewBox="0 0 24 24"
													>
														<path
															stroke-linecap="round"
															stroke-linejoin="round"
															stroke-width="2"
															d="M6 18L18 6M6 6l12 12"
														/>
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

							<!-- 弹幕设置 -->
							<div class="space-y-6">
								<h2 class="text-lg font-semibold">弹幕设置</h2>

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
							</div>

							<!-- 并发控制设置 -->
							<div class="space-y-6">
								<h2 class="text-lg font-semibold">并发控制设置</h2>

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

							<!-- 其他设置 -->
							<div class="space-y-6">
								<h2 class="text-lg font-semibold">其他设置</h2>

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
							</div>

							<!-- 提交按钮 -->
							<div class="flex {isMobile ? 'flex-col' : ''} gap-2 border-t pt-4">
								<Button type="submit" disabled={saving} class={isMobile ? 'w-full' : ''}>
									{saving ? '保存中...' : '保存设置'}
								</Button>
								<Button
									type="button"
									variant="outline"
									onclick={loadConfig}
									class={isMobile ? 'w-full' : ''}
								>
									重置
								</Button>
							</div>
						</form>
					</div>

					<!-- 右侧：变量说明 -->
					{#if showHelp}
						<div class={isMobile ? 'mt-6 w-full' : 'flex-1'}>
							<div
								class="rounded-lg border bg-white {isMobile
									? ''
									: 'h-full'} flex flex-col overflow-hidden {isMobile
									? ''
									: 'sticky top-6'} max-h-[calc(100vh-200px)]"
							>
								<div class="border-b bg-gray-50 p-4">
									<h3 class="text-base font-medium">📖 配置说明与模板变量</h3>
								</div>

								<div class="flex-1 overflow-y-auto p-4">
									<div class="grid grid-cols-1 gap-6">
										<!-- 配置项说明 -->
										<div>
											<h4 class="mb-3 font-medium text-red-600">🛠️ 配置项说明</h4>
											<div class="space-y-4 text-sm">
												<div class="rounded-lg border border-red-200 bg-red-50 p-3">
													<h5 class="mb-2 font-medium text-red-800">文件命名模板</h5>
													<div class="space-y-1 text-red-700">
														<p>
															<strong>video_name：</strong
															>视频文件夹名称，支持路径分隔符实现分类存储
														</p>
														<p><strong>page_name：</strong>单P视频文件名</p>
														<p>
															<strong>multi_page_name：</strong>多P视频文件名，必须包含分页标识符
														</p>
														<p><strong>bangumi_name：</strong>番剧文件名，建议使用 S01E01 格式</p>
														<p><strong>folder_structure：</strong>文件夹结构模板</p>
													</div>
												</div>

												<div class="rounded-lg border border-blue-200 bg-blue-50 p-3">
													<h5 class="mb-2 font-medium text-blue-800">视频质量过滤</h5>
													<div class="space-y-1 text-blue-700">
														<p>
															<strong>视频质量范围：</strong>8K > 4K > 1080P+ > 1080P60 > 1080P >
															720P60 > 720P > 480P > 360P
														</p>
														<p>
															<strong>音频质量范围：</strong>Hi-Res > 320k > 128k >
															64k，设置范围避免筛选不到符合要求的流
														</p>
														<p><strong>编解码器优先级：</strong></p>
														<p class="ml-3">• AVC(H.264): 兼容性最佳，硬解支持广泛，文件较大</p>
														<p class="ml-3">• HEV(H.265): 压缩率更高，需要较新设备硬解支持</p>
														<p class="ml-3">• AV1: 最新编码，压缩率最高，需要最新硬件支持</p>
														<p>
															<strong>杜比/HDR选项：</strong
															>杜比视界、杜比全景声、HDR视频流、Hi-Res音频流开关
														</p>
													</div>
												</div>

												<div class="rounded-lg border border-green-200 bg-green-50 p-3">
													<h5 class="mb-2 font-medium text-green-800">弹幕设置</h5>
													<div class="space-y-1 text-green-700">
														<p><strong>持续时间：</strong>弹幕在屏幕上显示的时间（秒）</p>
														<p><strong>字体样式：</strong>字体、大小、加粗、描边等外观设置</p>
														<p><strong>布局设置：</strong>轨道高度、间距、占比等位置控制</p>
														<p><strong>时间偏移：</strong>正值延后弹幕，负值提前弹幕</p>
													</div>
												</div>

												<div class="rounded-lg border border-purple-200 bg-purple-50 p-3">
													<h5 class="mb-2 font-medium text-purple-800">并发控制</h5>
													<div class="space-y-1 text-purple-700">
														<p><strong>视频并发数：</strong>同时处理的视频数量（建议1-5）</p>
														<p><strong>分页并发数：</strong>每个视频内的并发分页数（建议1-3）</p>
														<p>
															<strong>请求频率限制：</strong
															>防止API请求过频繁导致风控，调小limit可减少被限制
														</p>
														<p><strong>总并行度：</strong>约等于 视频并发数 × 分页并发数</p>
													</div>
												</div>

												<div class="rounded-lg border border-orange-200 bg-orange-50 p-3">
													<h5 class="mb-2 font-medium text-orange-800">其他设置</h5>
													<div class="space-y-1 text-orange-700">
														<p><strong>扫描间隔：</strong>每次扫描下载的时间间隔（秒）</p>
														<p>
															<strong>NFO时间类型：</strong>favtime（收藏时间）或
															pubtime（发布时间）
														</p>
														<p><strong>时间格式：</strong>控制时间变量在文件名中的显示格式</p>
														<p>
															<strong>CDN排序：</strong
															>启用后优先使用质量更高的CDN，可能提升下载速度
														</p>
														<p><strong>多线程下载：</strong>启用aria2多线程下载功能</p>
													</div>
												</div>
											</div>
										</div>

										<!-- 模板变量说明 -->
										<div>
											<h4 class="mb-2 font-medium text-blue-600">🎬 视频变量</h4>
											<div class="space-y-1 text-sm">
												{#each variableHelp.video as variable}
													<div class="flex">
														<code
															class="mr-2 min-w-fit rounded bg-blue-50 px-2 py-0.5 text-xs text-blue-700"
															>{variable.name}</code
														>
														<span class="text-xs text-gray-600">{variable.desc}</span>
													</div>
												{/each}
											</div>
										</div>

										<div>
											<h4 class="mb-2 font-medium text-green-600">📄 分页变量</h4>
											<div class="space-y-1 text-sm">
												{#each variableHelp.page as variable}
													<div class="flex">
														<code
															class="mr-2 min-w-fit rounded bg-green-50 px-2 py-0.5 text-xs text-green-700"
															>{variable.name}</code
														>
														<span class="text-xs text-gray-600">{variable.desc}</span>
													</div>
												{/each}
											</div>
										</div>

										<div>
											<h4 class="mb-2 font-medium text-purple-600">🔧 通用功能</h4>
											<div class="space-y-1 text-sm">
												{#each variableHelp.common as variable}
													<div class="flex">
														<code
															class="mr-2 min-w-fit rounded bg-purple-50 px-2 py-0.5 text-xs text-purple-700"
															>{variable.name}</code
														>
														<span class="text-xs text-gray-600">{variable.desc}</span>
													</div>
												{/each}
											</div>
										</div>

										<div>
											<h4 class="mb-2 font-medium text-orange-600">⏰ 时间格式</h4>
											<div class="space-y-1 text-sm">
												{#each variableHelp.time as variable}
													<div class="flex">
														<code
															class="mr-2 min-w-fit rounded bg-orange-50 px-2 py-0.5 text-xs text-orange-700"
															>{variable.name}</code
														>
														<span class="text-xs text-gray-600">{variable.desc}</span>
													</div>
												{/each}
											</div>
										</div>
									</div>

									<div class="mt-6 rounded-lg border border-blue-200 bg-blue-50 p-3">
										<h4 class="mb-3 text-sm font-medium text-blue-800">💡 使用示例</h4>
										<div class="space-y-3 text-xs text-blue-700">
											<div>
												<strong>视频命名模板：</strong>
												<div class="mt-1 ml-4 space-y-1">
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'{{upper_name}} - {{title}}'}</code
														>
														<span class="text-gray-600"
															>→ 庄心妍 - 没想到吧～这些歌原来是我唱的！</span
														>
													</div>
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'{{title}} [{{bvid}}]'}</code
														>
														<span class="text-gray-600"
															>→ 【觅长生】废人修仙传#01 [BV1abc123def]</span
														>
													</div>
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'{{upper_name}}/{{title}}_{{pubtime}}'}</code
														>
														<span class="text-gray-600">→ 庄心妍/庄心妍的街头采访_2023-12-25</span>
													</div>
												</div>
											</div>
											<div>
												<strong>分页命名模板：</strong>
												<div class="mt-1 ml-4 space-y-1">
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'{{title}}'}</code
														>
														<span class="text-gray-600">→ 庄心妍的街头采访（单P视频）</span>
													</div>
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'{{ptitle}}'}</code
														>
														<span class="text-gray-600">→ 使用分页标题</span>
													</div>
												</div>
											</div>
											<div>
												<strong>多P视频命名模板：</strong>
												<div class="mt-1 ml-4 space-y-1">
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'{{title}}-P{{pid_pad}}'}</code
														>
														<span class="text-gray-600">→ 视频标题-P001.mp4（推荐格式）</span>
													</div>
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'S{{season_pad}}E{{pid_pad}}-{{pid_pad}}'}</code
														>
														<span class="text-gray-600">→ S01E01-01.mp4（番剧格式）</span>
													</div>
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'{{ptitle}}'}</code
														>
														<span class="text-gray-600">→ 使用分页标题命名</span>
													</div>
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'第{{pid}}集'}</code
														>
														<span class="text-gray-600">→ 第1集.mp4、第2集.mp4</span>
													</div>
												</div>
											</div>
											<div>
												<strong>番剧命名模板：</strong>
												<div class="mt-1 ml-4 space-y-1">
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'S{{season_pad}}E{{pid_pad}}-{{pid_pad}}'}</code
														>
														<span class="text-gray-600">→ S01E01-01.mp4（标准番剧格式）</span>
													</div>
												</div>
											</div>
											<div>
												<strong>文件夹结构模板：</strong>
												<div class="mt-1 ml-4 space-y-1">
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>Season 1</code
														>
														<span class="text-gray-600">→ 多P视频的分季文件夹</span>
													</div>
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'第{{pid}}季'}</code
														>
														<span class="text-gray-600">→ 第1季、第2季...</span>
													</div>
												</div>
											</div>
											<div>
												<strong>时间格式示例：</strong>
												<div class="mt-1 ml-4 space-y-1">
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>%Y-%m-%d</code
														>
														<span class="text-gray-600">→ 2023-12-25</span>
													</div>
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>%Y年%m月%d日</code
														>
														<span class="text-gray-600">→ 2023年12月25日</span>
													</div>
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>%Y-%m-%d %H:%M</code
														>
														<span class="text-gray-600">→ 2023-12-25 14:30</span>
													</div>
												</div>
											</div>
											<div>
												<strong>截取函数示例：</strong>
												<div class="mt-1 ml-4 space-y-1">
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'{{ truncate title 20 }}'}</code
														>
														<span class="text-gray-600">→ 截取标题前20个字符</span>
													</div>
													<div class="flex items-start">
														<code class="mr-2 rounded border border-blue-200 bg-white px-2 py-0.5"
															>{'{{ truncate upper_name 10 }} - {{title}}'}</code
														>
														<span class="text-gray-600">→ 截取UP主名前10个字符</span>
													</div>
												</div>
											</div>
										</div>
									</div>
								</div>
							</div>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
