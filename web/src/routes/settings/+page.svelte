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
	let parallelDownloadMinSize = 10485760;

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

	// 响应式相关
	let innerWidth: number;
	let isMobile: boolean = false;
	$: isMobile = innerWidth < 768; // md断点

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
			parallelDownloadMinSize = config.parallel_download_min_size || 10485760;
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
				parallel_download_min_size: parallelDownloadMinSize
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
		<div class="bg-card rounded-lg shadow-sm border p-6">
			<h1 class="text-2xl font-bold mb-6">系统设置</h1>

			{#if loading}
				<div class="flex items-center justify-center py-12">
					<div class="text-muted-foreground">加载中...</div>
				</div>
			{:else}
				<div class="flex {isMobile ? 'flex-col' : 'gap-8'}">
					<!-- 左侧：表单区域 -->
					<div class="{isMobile ? 'w-full' : 'w-[600px] flex-shrink-0'}">
						<form onsubmit={(e) => { e.preventDefault(); saveConfig(); }} class="space-y-8">
							<!-- 文件命名模板 -->
							<div class="space-y-6">
								<div class="flex {isMobile ? 'flex-col gap-2' : 'justify-between items-center'}">
									<h2 class="text-lg font-semibold">文件命名模板</h2>
									<Button 
										type="button" 
										variant="outline"
										size="sm"
										onclick={() => showHelp = !showHelp}
										class="{isMobile ? 'w-full' : ''}"
									>
										{showHelp ? '隐藏' : '显示'}变量说明
									</Button>
								</div>
								
								<div class="space-y-2">
									<Label for="video-name">视频文件名</Label>
									<Input 
										id="video-name" 
										bind:value={videoName} 
										placeholder={'{{title}}'}
									/>
									<p class="text-sm text-muted-foreground">可用变量：&#123;&#123;title&#125;&#125;, &#123;&#123;bvid&#125;&#125;, &#123;&#123;avid&#125;&#125;</p>
								</div>

								<div class="space-y-2">
									<Label for="page-name">分P视频名</Label>
									<Input 
										id="page-name" 
										bind:value={pageName} 
										placeholder={'{{title}}'}
									/>
									<p class="text-sm text-muted-foreground">单P视频的命名模板</p>
								</div>

								<div class="space-y-2">
									<Label for="multi-page-name">多P视频名</Label>
									<Input 
										id="multi-page-name" 
										bind:value={multiPageName} 
										placeholder={'{{title}}-P{{pid_pad}}'}
									/>
									<p class="text-sm text-muted-foreground">多P视频必须包含分页标识符，如 &#123;&#123;pid&#125;&#125; 或 &#123;&#123;pid_pad&#125;&#125;</p>
								</div>

								<div class="space-y-2">
									<Label for="bangumi-name">番剧文件名</Label>
									<Input 
										id="bangumi-name" 
										bind:value={bangumiName} 
										placeholder={'S{{season_pad}}E{{pid_pad}}-{{pid_pad}}'}
									/>
									<p class="text-sm text-muted-foreground">番剧专用模板，必须包含分页标识符</p>
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
									<Input 
										id="time-format" 
										bind:value={timeFormat} 
										placeholder="%Y-%m-%d"
									/>
									<p class="text-sm text-muted-foreground">Python strftime 格式</p>
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
										class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
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
										class="h-4 w-4 rounded border-gray-300 text-primary focus:ring-primary"
									/>
									<Label 
										for="parallel-download" 
										class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
									>
										启用多线程下载
									</Label>
								</div>

								{#if parallelDownloadEnabled}
									<div class="space-y-2 ml-6">
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

									<div class="space-y-2 ml-6">
										<Label for="min-size">最小文件大小（字节）</Label>
										<Input 
											id="min-size" 
											type="number"
											bind:value={parallelDownloadMinSize} 
											min="0"
											placeholder="10485760"
										/>
										<p class="text-sm text-muted-foreground">小于此大小的文件不使用多线程下载（默认 10MB）</p>
									</div>
								{/if}
							</div>

							<!-- 提交按钮 -->
							<div class="flex {isMobile ? 'flex-col' : ''} gap-2 pt-4 border-t">
								<Button type="submit" disabled={saving} class="{isMobile ? 'w-full' : ''}">
									{saving ? '保存中...' : '保存设置'}
								</Button>
								<Button type="button" variant="outline" onclick={loadConfig} class="{isMobile ? 'w-full' : ''}">
									重置
								</Button>
							</div>
						</form>
					</div>

					<!-- 右侧：变量说明 -->
					{#if showHelp}
						<div class="{isMobile ? 'w-full mt-6' : 'flex-1'}">
							<div class="bg-white rounded-lg border {isMobile ? '' : 'h-full'} overflow-hidden flex flex-col {isMobile ? '' : 'sticky top-6'} max-h-[calc(100vh-200px)]">
								<div class="p-4 border-b bg-gray-50">
									<h3 class="text-base font-medium">📝 支持的模板变量</h3>
								</div>
								
								<div class="flex-1 overflow-y-auto p-4">
									<div class="grid grid-cols-1 gap-4">
										<div>
											<h4 class="font-medium text-blue-600 mb-2">🎬 视频变量</h4>
											<div class="space-y-1 text-sm">
												{#each variableHelp.video as variable}
													<div class="flex">
														<code class="bg-blue-50 px-2 py-0.5 rounded text-blue-700 mr-2 min-w-fit text-xs">{variable.name}</code>
														<span class="text-gray-600 text-xs">{variable.desc}</span>
													</div>
												{/each}
											</div>
										</div>
										
										<div>
											<h4 class="font-medium text-green-600 mb-2">📄 分页变量</h4>
											<div class="space-y-1 text-sm">
												{#each variableHelp.page as variable}
													<div class="flex">
														<code class="bg-green-50 px-2 py-0.5 rounded text-green-700 mr-2 min-w-fit text-xs">{variable.name}</code>
														<span class="text-gray-600 text-xs">{variable.desc}</span>
													</div>
												{/each}
											</div>
										</div>
										
										<div>
											<h4 class="font-medium text-purple-600 mb-2">🔧 通用功能</h4>
											<div class="space-y-1 text-sm">
												{#each variableHelp.common as variable}
													<div class="flex">
														<code class="bg-purple-50 px-2 py-0.5 rounded text-purple-700 mr-2 min-w-fit text-xs">{variable.name}</code>
														<span class="text-gray-600 text-xs">{variable.desc}</span>
													</div>
												{/each}
											</div>
										</div>
										
										<div>
											<h4 class="font-medium text-orange-600 mb-2">⏰ 时间格式</h4>
											<div class="space-y-1 text-sm">
												{#each variableHelp.time as variable}
													<div class="flex">
														<code class="bg-orange-50 px-2 py-0.5 rounded text-orange-700 mr-2 min-w-fit text-xs">{variable.name}</code>
														<span class="text-gray-600 text-xs">{variable.desc}</span>
													</div>
												{/each}
											</div>
										</div>
									</div>
									
									<div class="mt-6 p-3 bg-blue-50 rounded-lg border border-blue-200">
										<h4 class="font-medium text-blue-800 mb-3 text-sm">💡 使用示例</h4>
										<div class="text-xs text-blue-700 space-y-3">
											<div>
												<strong>视频命名模板：</strong>
												<div class="ml-4 space-y-1 mt-1">
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'{{upper_name}} - {{title}}'}</code>
														<span class="text-gray-600">→ 庄心妍 - 没想到吧～这些歌原来是我唱的！</span>
													</div>
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'{{title}} [{{bvid}}]'}</code>
														<span class="text-gray-600">→ 【觅长生】废人修仙传#01 [BV1abc123def]</span>
													</div>
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'{{upper_name}}/{{title}}_{{pubtime}}'}</code>
														<span class="text-gray-600">→ 庄心妍/庄心妍的街头采访_2023-12-25</span>
													</div>
												</div>
											</div>
											<div>
												<strong>分页命名模板：</strong>
												<div class="ml-4 space-y-1 mt-1">
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'{{title}}'}</code>
														<span class="text-gray-600">→ 庄心妍的街头采访（单P视频）</span>
													</div>
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'{{ptitle}}'}</code>
														<span class="text-gray-600">→ 使用分页标题</span>
													</div>
												</div>
											</div>
											<div>
												<strong>多P视频命名模板：</strong>
												<div class="ml-4 space-y-1 mt-1">
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'{{title}}-P{{pid_pad}}'}</code>
														<span class="text-gray-600">→ 视频标题-P001.mp4（推荐格式）</span>
													</div>
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'S{{season_pad}}E{{pid_pad}}-{{pid_pad}}'}</code>
														<span class="text-gray-600">→ S01E01-01.mp4（番剧格式）</span>
													</div>
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'{{ptitle}}'}</code>
														<span class="text-gray-600">→ 使用分页标题命名</span>
													</div>
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'第{{pid}}集'}</code>
														<span class="text-gray-600">→ 第1集.mp4、第2集.mp4</span>
													</div>
												</div>
											</div>
											<div>
												<strong>番剧命名模板：</strong>
												<div class="ml-4 space-y-1 mt-1">
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'S{{season_pad}}E{{pid_pad}}-{{pid_pad}}'}</code>
														<span class="text-gray-600">→ S01E01-01.mp4（标准番剧格式）</span>
													</div>
												</div>
											</div>
											<div>
												<strong>文件夹结构模板：</strong>
												<div class="ml-4 space-y-1 mt-1">
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">Season 1</code>
														<span class="text-gray-600">→ 多P视频的分季文件夹</span>
													</div>
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'第{{pid}}季'}</code>
														<span class="text-gray-600">→ 第1季、第2季...</span>
													</div>
												</div>
											</div>
											<div>
												<strong>时间格式示例：</strong>
												<div class="ml-4 space-y-1 mt-1">
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">%Y-%m-%d</code>
														<span class="text-gray-600">→ 2023-12-25</span>
													</div>
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">%Y年%m月%d日</code>
														<span class="text-gray-600">→ 2023年12月25日</span>
													</div>
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">%Y-%m-%d %H:%M</code>
														<span class="text-gray-600">→ 2023-12-25 14:30</span>
													</div>
												</div>
											</div>
											<div>
												<strong>截取函数示例：</strong>
												<div class="ml-4 space-y-1 mt-1">
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'{{ truncate title 20 }}'}</code>
														<span class="text-gray-600">→ 截取标题前20个字符</span>
													</div>
													<div class="flex items-start">
														<code class="bg-white px-2 py-0.5 rounded border border-blue-200 mr-2">{'{{ truncate upper_name 10 }} - {{title}}'}</code>
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
 