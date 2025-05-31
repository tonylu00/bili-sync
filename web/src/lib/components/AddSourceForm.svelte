<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import { addVideoSource, getBangumiSeasons, searchBilibili, getUserFavorites, getUserCollections } from '$lib/api';
	import { toast } from 'svelte-sonner';
	import type { VideoCategory, UserFavoriteFolder } from '$lib/types';
	import { createEventDispatcher } from 'svelte';

	export let onSuccess: () => void;

	const dispatch = createEventDispatcher();

	let source_type: VideoCategory = 'collection';
	let source_id = '';
	let up_id = '';
	let name = '';
	let path = '/Downloads';
	let download_all_seasons = false;
	let collection_type = 'season';
	let loading = false;
	
	// 番剧季度相关
	let loadingSeasons = false;
	let seasons: Array<{
		season_id: string;
		season_title: string;
		media_id?: string;
		cover?: string;
		full_title?: string;
	}> = [];
	let selectedSeasons: string[] = [];
	
	// 搜索相关 - 简化，只保留搜索输入和加载状态
	let searchKeyword = '';
	let searchLoading = false;
	
	// 用户收藏夹相关
	let userFavorites: UserFavoriteFolder[] = [];
	let loadingFavorites = false;
	let showFavorites = false;
	
	// UP主合集相关
	let userCollections: Array<{
		collection_type: string;
		sid: string;
		name: string;
		cover: string;
		description: string;
		total: number;
		ptime?: number;
		mid: number;
	}> = [];
	let loadingCollections = false;
	let showCollections = false;
	let isManualInput = false; // 标记是否手动输入合集ID
	
	// 源类型对应的中文名称和说明
	const sourceTypeLabels = {
		collection: { name: '合集', description: '合集ID可在合集页面URL中获取' },
		favorite: { name: '收藏夹', description: '收藏夹ID可在收藏夹页面URL中获取' },
		submission: { name: 'UP主投稿', description: 'UP主ID可在UP主空间URL中获取' },
		watch_later: { name: '稍后观看', description: '只能添加一个稍后观看源' },
		bangumi: { name: '番剧', description: '番剧season_id可在番剧页面URL中获取' }
	};
	
	// 合集类型对应的中文名称和说明
	const collectionTypeLabels: {
		[key: string]: { name: string; description: string };
		season: { name: string; description: string };
		series: { name: string; description: string };
	} = {
		season: { name: '合集', description: 'B站标准合集，有统一的合集页面和标题-season:{mid}:{season_id}' },
		series: { name: '列表', description: '视频列表，组织较松散的视频合集-series:{mid}:{series_id}' }
	};

	// 获取番剧的所有季度信息
	async function fetchBangumiSeasons() {
		if (!source_id || source_type !== 'bangumi') return;
		
		loadingSeasons = true;
		try {
			const result = await getBangumiSeasons(source_id);
			if (result.success && result.data) {
				seasons = result.data;
				// 默认不选中任何季度
				selectedSeasons = [];
			}
		} catch (error) {
			console.error('获取季度信息失败:', error);
			seasons = [];
			selectedSeasons = [];
		} finally {
			loadingSeasons = false;
		}
	}
	
	// 监听 source_id 变化，自动获取季度信息
	$: if (source_type === 'bangumi' && source_id) {
		fetchBangumiSeasons();
	}
	
	// 当切换视频源类型时，清空季度相关状态
	$: if (source_type !== 'bangumi') {
		seasons = [];
		selectedSeasons = [];
	}
	
	// 当切换到收藏夹类型时，自动获取用户收藏夹列表
	$: if (source_type === 'favorite') {
		fetchUserFavorites();
	} else {
		showFavorites = false;
		userFavorites = [];
	}
	
	// 切换季度选择
	function toggleSeasonSelection(seasonId: string) {
		const index = selectedSeasons.indexOf(seasonId);
		if (index === -1) {
			selectedSeasons = [...selectedSeasons, seasonId];
		} else {
			selectedSeasons = selectedSeasons.filter(id => id !== seasonId);
		}
	}
	
	// 获取用户收藏夹列表
	async function fetchUserFavorites() {
		if (source_type !== 'favorite') return;
		
		loadingFavorites = true;
		try {
			userFavorites = await getUserFavorites();
			showFavorites = true;
		} catch (error) {
			console.error('获取收藏夹列表失败:', error);
			toast.error('获取收藏夹列表失败', { description: `错误信息：${error}` });
			userFavorites = [];
			showFavorites = false;
		} finally {
			loadingFavorites = false;
		}
	}

	// 获取UP主合集列表
	async function fetchUserCollections() {
		if (source_type !== 'collection' || !up_id) return;
		
		loadingCollections = true;
		try {
			const result = await getUserCollections(up_id);
			if (result.success) {
				userCollections = result.collections;
				showCollections = true;
			}
		} catch (error) {
			console.error('获取UP主合集列表失败:', error);
			toast.error('获取UP主合集列表失败', { description: `错误信息：${error}` });
			userCollections = [];
			showCollections = false;
		} finally {
			loadingCollections = false;
		}
	}

	// 选择收藏夹
	function selectFavorite(favorite: UserFavoriteFolder) {
		source_id = favorite.id;
		name = favorite.title;
		showFavorites = false;
		toast.success('已选择收藏夹', { description: favorite.title });
	}

	// 选择合集
	function selectCollection(collection: any) {
		source_id = collection.sid;
		name = collection.name;
		collection_type = collection.collection_type;
		showCollections = false;
		isManualInput = false; // 从列表选择，非手动输入
		toast.success('已选择合集', { description: `${collection.collection_type === 'season' ? '合集' : '系列'}：${collection.name}` });
	}

	// 监听UP主ID变化，自动获取合集列表（添加防抖）
	let upIdTimeout: number;
	$: if (source_type === 'collection' && up_id) {
		clearTimeout(upIdTimeout);
		upIdTimeout = setTimeout(() => {
			if (up_id.trim()) {
				fetchUserCollections();
			}
		}, 500); // 500ms防抖
	} else if (source_type !== 'collection') {
		showCollections = false;
		userCollections = [];
	}

	// 搜索bilibili内容 - 修改为通过事件分发搜索结果
	async function handleSearch(isNewSearch = true, page = 1) {
		if (!searchKeyword.trim()) {
			toast.error('请输入搜索关键词');
			return;
		}

		// 根据当前选择的视频源类型确定搜索类型
		let searchType: 'video' | 'bili_user' | 'media_bangumi';
		switch (source_type) {
			case 'submission':
				searchType = 'bili_user';
				break;
			case 'bangumi':
				searchType = 'media_bangumi';
				break;
			default:
				searchType = 'video';
				break;
		}

		console.log('开始搜索:', { keyword: searchKeyword, searchType, page });
		searchLoading = true;
		
		try {
			const result = await searchBilibili({
				keyword: searchKeyword,
				search_type: searchType,
				page: page,
				page_size: 12  // 每页显示12个
			});

			console.log('搜索成功，结果:', result);
			
			if (result.success) {
				// 通过事件将搜索结果传递给父组件
				dispatch('searchResults', {
					results: result.results,
					total: result.total,
					keyword: searchKeyword,
					searchType: searchType,
					sourceType: source_type,
					page: page
				});
			} else {
				console.error('搜索返回失败状态:', result);
				toast.error('搜索失败');
			}
		} catch (error) {
			console.error('搜索请求异常:', error);
			toast.error('搜索失败', { description: `错误信息：${error}` });
		} finally {
			searchLoading = false;
		}
	}

	// 带页码的搜索（供父组件调用）
	export async function searchWithPage(page: number) {
		await handleSearch(false, page);
	}

	// 填充搜索结果到表单 - 由父组件调用
	export function fillFromSearchResult(result: any, sourceType: VideoCategory) {
		console.log('填充搜索结果:', result, sourceType);
		
		try {
			switch (sourceType) {
				case 'submission':
					if (result.mid) {
						source_id = result.mid.toString();
						name = cleanTitle(result.title);
					}
					break;
				case 'bangumi':
					// 处理番剧和影视类型
					if (result.result_type === 'media_bangumi' || result.result_type === 'media_ft') {
						if (result.season_id) {
							source_id = result.season_id;
							name = cleanTitle(result.title);
						}
					}
					break;
				case 'collection':
				case 'favorite':
				default:
					if (result.bvid) {
						source_id = result.bvid;
						name = cleanTitle(result.title);
					}
					break;
			}
			
			// 清空搜索
			searchKeyword = '';
			
			toast.success('已填充信息', { description: '请检查并完善其他必要信息' });
		} catch (error) {
			console.error('填充搜索结果时出错:', error);
			toast.error('填充失败');
		}
	}

	// 清理标题中的HTML标签
	function cleanTitle(title: string): string {
		// 移除HTML标签并解码HTML实体
		const div = document.createElement('div');
		div.innerHTML = title;
		return div.textContent || div.innerText || title;
	}

	// 处理B站图片URL，确保格式正确
	function processBilibiliImageUrl(url: string): string {
		if (!url) return '';
		
		// 如果已经是完整的HTTPS URL，直接返回
		if (url.startsWith('https://')) {
			return url;
		}
		
		// 处理以 // 开头的URL
		if (url.startsWith('//')) {
			url = 'https:' + url;
		}
		
		// 处理以 http:// 开头的URL，替换为 https://
		if (url.startsWith('http://')) {
			url = url.replace('http://', 'https://');
		}
		
		// 如果URL不包含协议，添加https
		if (!url.startsWith('http')) {
			url = 'https://' + url;
		}
		
		// 移除已有的图片参数，使用原图
		if (url.includes('@')) {
			url = url.split('@')[0];
		}
		
		return url;
	}

	// 处理图片加载失败
	function handleImageError(event: Event) {
		const img = event.target as HTMLImageElement;
		// 可以设置一个默认图片或隐藏图片容器
		const parent = img.parentElement;
		if (parent) {
			parent.innerHTML = '<span class="text-xs text-gray-400">无封面</span>';
			parent.classList.add('flex', 'items-center', 'justify-center');
		}
	}

	async function handleSubmit() {
		if (source_type !== 'watch_later' && !source_id) {
			// 所有类型（除稍后观看外）都需要source_id
			toast.error('请输入ID', { description: '视频源ID不能为空' });
			return;
		}
		
		if (source_type === 'collection' && !up_id) {
			toast.error('请输入UP主ID', { description: '合集需要提供UP主ID' });
			return;
		}
		
		if (!name) {
			toast.error('请输入名称', { description: '视频源名称不能为空' });
			return;
		}
		
		if (!path) {
			toast.error('请输入保存路径', { description: '保存路径不能为空' });
			return;
		}
		
		loading = true;
		
		try {
			const result = await addVideoSource({
				source_type,
				source_id,
				up_id: source_type === 'collection' ? up_id : undefined,
				name,
				path,
				collection_type: source_type === 'collection' ? collection_type : undefined,
				download_all_seasons: source_type === 'bangumi' ? download_all_seasons : undefined,
				selected_seasons: source_type === 'bangumi' && selectedSeasons.length > 0 ? selectedSeasons : undefined
			});
			
			if (result.success) {
				toast.success('添加成功', { description: result.message });
				// 重置表单
				source_id = '';
				up_id = '';
				name = '';
				path = '/Downloads';
				download_all_seasons = false;
				collection_type = 'season';
				isManualInput = false;
				// 重置季度选择状态
				seasons = [];
				selectedSeasons = [];
				// 调用成功回调，通知父组件刷新数据
				onSuccess();
			} else {
				toast.error('添加失败', { description: result.message });
			}
		} catch (error) {
			console.error(error);
			toast.error('添加失败', { description: `错误信息：${error}` });
		} finally {
			loading = false;
		}
	}
</script>

<div class="bg-white p-4 rounded shadow-md">
	<h2 class="text-xl font-bold mb-4">添加新视频源</h2>
	
	<form on:submit|preventDefault={handleSubmit} class="space-y-4">
		<div>
			<label class="block text-sm font-medium mb-1" for="source-type">
				视频源类型
			</label>
			<select 
				id="source-type" 
				class="w-full p-2 border rounded bg-gray-50 text-gray-900 border-gray-300 focus:border-blue-500 focus:ring-1 focus:ring-blue-500" 
				bind:value={source_type}
			>
				<option value="collection">合集</option>
				<option value="favorite">收藏夹</option>
				<option value="submission">UP主投稿</option>
				<option value="watch_later">稍后观看</option>
				<option value="bangumi">番剧</option>
			</select>
			<p class="text-xs text-gray-500 mt-1">{sourceTypeLabels[source_type].description}</p>
		</div>
		
		{#if source_type === 'collection' && isManualInput}
		<div>
			<label class="block text-sm font-medium mb-1" for="collection-type">
				合集类型
			</label>
			<select 
				id="collection-type" 
				class="w-full p-2 border rounded bg-gray-50 text-gray-900 border-gray-300 focus:border-blue-500 focus:ring-1 focus:ring-blue-500" 
				bind:value={collection_type}
			>
				<option value="season">{collectionTypeLabels.season.name}</option>
				<option value="series">{collectionTypeLabels.series.name}</option>
			</select>
			<p class="text-xs text-gray-500 mt-1">{collectionTypeLabels[collection_type].description}</p>
			<p class="text-xs text-orange-600 mt-1">⚠️ 手动输入合集ID时需要指定类型，建议从下方UP主合集列表中选择</p>
		</div>
		{/if}
		
		{#if source_type === 'collection'}
		<div>
			<label class="block text-sm font-medium mb-1" for="up-id">
				UP主ID
			</label>
			<Input id="up-id" bind:value={up_id} placeholder="请输入UP主ID（可在UP主空间URL中获取）" />
			<p class="text-xs text-gray-500 mt-1">UP主ID是合集所属UP主的唯一标识，必须提供</p>
		</div>
		{/if}
		
		{#if source_type !== 'watch_later'}
		<!-- 搜索功能 / 收藏夹列表 / 合集列表 -->
		{#if source_type === 'favorite'}
		<!-- 收藏夹列表 -->
		<div class="bg-blue-50 p-3 rounded border">
			<div class="flex items-center gap-2 mb-2">
				<span class="text-sm font-medium text-blue-700">📁 我的收藏夹</span>
				<span class="text-xs text-blue-600">
					选择要同步的收藏夹
				</span>
			</div>
			
			{#if loadingFavorites}
				<div class="p-3 text-center text-gray-500 text-sm">
					正在加载收藏夹列表...
				</div>
			{:else if showFavorites}
				<div class="border rounded bg-white max-h-80 overflow-hidden flex flex-col">
					<div class="flex justify-between items-center p-2 border-b bg-gray-50 flex-shrink-0">
						<span class="text-sm font-medium">收藏夹列表 (共{userFavorites.length}个)</span>
						<button 
							type="button" 
							on:click={() => showFavorites = false}
							class="text-gray-500 hover:text-gray-700 text-sm"
						>
							✕
						</button>
					</div>
					
					{#if userFavorites.length === 0}
						<div class="p-3 text-center text-gray-500 text-sm">
							没有找到收藏夹
						</div>
					{:else}
						<!-- 收藏夹列表 -->
						<div class="flex-1 overflow-y-auto">
							{#each userFavorites as favorite}
							<button 
								type="button"
								class="w-full p-3 border-b last:border-b-0 hover:bg-blue-50 text-left transition-colors"
								on:click={() => selectFavorite(favorite)}
							>
								<div class="flex items-center justify-between">
									<div class="flex-1 min-w-0">
										<h4 class="font-medium text-sm text-gray-900 mb-1 truncate">
											{favorite.title}
										</h4>
										<p class="text-xs text-gray-600">
											收藏夹ID: {favorite.id} | {favorite.media_count} 个视频
										</p>
									</div>
									<div class="ml-2 text-xs text-blue-600">
										选择
									</div>
								</div>
							</button>
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		</div>
		{:else if source_type === 'collection'}
		<!-- UP主合集列表 -->
		<div class="bg-blue-50 p-3 rounded border">
			<div class="flex items-center gap-2 mb-2">
				<span class="text-sm font-medium text-blue-700">📚 UP主合集</span>
				<span class="text-xs text-blue-600">
					{#if up_id}
						输入UP主ID后自动显示该UP主的合集和系列
					{:else}
						请先输入UP主ID
					{/if}
				</span>
			</div>
			
			{#if loadingCollections}
				<div class="p-3 text-center text-gray-500 text-sm">
					正在加载UP主合集列表...
				</div>
			{:else if showCollections}
				<div class="border rounded bg-white max-h-80 overflow-hidden flex flex-col">
					<div class="flex justify-between items-center p-2 border-b bg-gray-50 flex-shrink-0">
						<span class="text-sm font-medium">合集列表 (共{userCollections.length}个)</span>
						<button 
							type="button" 
							on:click={() => showCollections = false}
							class="text-gray-500 hover:text-gray-700 text-sm"
						>
							✕
						</button>
					</div>
					
					{#if userCollections.length === 0}
						<div class="p-3 text-center text-gray-500 text-sm">
							该UP主没有合集或系列
						</div>
					{:else}
						<!-- 合集列表 -->
						<div class="flex-1 overflow-y-auto">
							{#each userCollections as collection}
							<button 
								type="button"
								class="w-full p-3 border-b last:border-b-0 hover:bg-blue-50 text-left transition-colors"
								on:click={() => selectCollection(collection)}
							>
								<div class="flex items-center">
									{#if collection.cover}
									<img 
										src={processBilibiliImageUrl(collection.cover)} 
										alt="封面" 
										class="w-16 h-10 object-cover rounded mr-3"
										on:error={handleImageError}
										loading="lazy"
										referrerpolicy="no-referrer"
										crossorigin="anonymous"
									/>
									{:else}
									<div class="w-16 h-10 bg-gray-200 rounded mr-3 flex items-center justify-center">
										<span class="text-xs text-gray-400">无封面</span>
									</div>
									{/if}
									<div class="flex-1 min-w-0">
										<div class="flex items-center gap-2 mb-1">
											<h4 class="font-medium text-sm text-gray-900 truncate">
												{collection.name}
											</h4>
											<span class="px-1.5 py-0.5 text-xs rounded {collection.collection_type === 'season' ? 'bg-green-100 text-green-700' : 'bg-blue-100 text-blue-700'}">
												{collection.collection_type === 'season' ? '合集' : '系列'}
											</span>
										</div>
										<p class="text-xs text-gray-600">
											ID: {collection.sid} | {collection.total} 个视频
										</p>
										{#if collection.description}
										<p class="text-xs text-gray-500 mt-1 line-clamp-1">
											{collection.description}
										</p>
										{/if}
									</div>
								</div>
							</button>
							{/each}
						</div>
					{/if}
				</div>
			{:else if up_id}
				<p class="text-xs text-gray-500">输入UP主ID后会自动加载合集列表</p>
			{/if}
		</div>
		{:else}
		<!-- 其他类型的搜索功能 - 简化版本 -->
		<div class="bg-blue-50 p-3 rounded border">
			<div class="flex items-center gap-2 mb-2">
				<span class="text-sm font-medium text-blue-700">🔍 智能搜索</span>
				<span class="text-xs text-blue-600">
					{source_type === 'submission' ? '搜索UP主' : 
					 source_type === 'bangumi' ? '搜索番剧和影视' : '搜索视频'}
				</span>
			</div>
			<div class="flex gap-2">
				<Input 
					bind:value={searchKeyword} 
					placeholder={source_type === 'submission' ? '输入UP主名称搜索...' : 
								source_type === 'bangumi' ? '输入番剧或影视名称搜索...' : '输入视频标题搜索...'}
					class="flex-1"
					on:keydown={(e) => {
						if (e.key === 'Enter') {
							e.preventDefault();
							handleSearch(true);
						}
					}}
				/>
				<button 
					type="button" 
					on:click={() => {
						console.log('搜索按钮被点击');
						handleSearch(true);
					}} 
					disabled={searchLoading || !searchKeyword.trim()}
					class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:bg-gray-300 disabled:cursor-not-allowed"
				>
					{searchLoading ? '搜索中...' : '搜索'}
				</button>
			</div>
			<p class="text-xs text-gray-500 mt-2">💡 搜索结果将在右侧显示，点击结果可自动填充表单</p>
		</div>
		{/if}
		
		{#if source_type !== 'favorite'}
		<div>
			<label class="block text-sm font-medium mb-1" for="source-id">
				{source_type === 'bangumi' ? 'season_id' : 
				  source_type === 'submission' ? 'UP主ID' : 
				  source_type === 'collection' ? '合集ID' : 'ID'}
			</label>
			<Input 
				id="source-id" 
				bind:value={source_id} 
				placeholder="请输入ID" 
				on:input={() => {
					if (source_type === 'collection') {
						isManualInput = true; // 手动输入时标记
					}
				}}
			/>
			{#if source_type === 'collection' && !isManualInput && source_id}
			<p class="text-xs text-green-600 mt-1">✓ 已从列表中选择合集，类型已自动识别</p>
			{/if}
		</div>
		{/if}
		{/if}
		
		<div>
			<label class="block text-sm font-medium mb-1" for="name">
				名称
			</label>
			<Input id="name" bind:value={name} placeholder="请输入名称，将显示在侧边栏" />
		</div>
		
		<div>
			<label class="block text-sm font-medium mb-1" for="path">
				保存路径
			</label>
			<Input id="path" bind:value={path} placeholder="请输入绝对路径，如: /Downloads" />
			<p class="text-xs text-gray-500 mt-1">必须是绝对路径，且有写入权限</p>
		</div>
		
		{#if source_type === 'bangumi'}
		<div class="flex items-center">
			<input 
				type="checkbox" 
				id="download-all-seasons" 
				bind:checked={download_all_seasons} 
				class="h-4 w-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
			/>
			<label for="download-all-seasons" class="ml-2 block text-sm text-gray-900">
				下载全部季度
			</label>
			<p class="text-xs text-gray-500 ml-2">启用后将下载该番剧的所有相关季度</p>
		</div>
		
		{#if !download_all_seasons && seasons.length > 0}
		<div>
			<div class="block text-sm font-medium mb-2">
				选择要下载的季度
				<span class="text-xs text-gray-500 ml-2">（不选择则下载当前输入的季度，不创建季度文件夹，会下载到设置的保存路径！！！注意这样的话在删除本视频源时会删除设置的保存路径下的所有文件！！！）</span>
			</div>
			{#if loadingSeasons}
				<p class="text-sm text-gray-500">正在加载季度信息...</p>
			{:else}
				<div class="space-y-2 max-h-60 overflow-y-auto border rounded p-2">
					{#each seasons as season}
						<div class="flex items-center">
							<input 
								type="checkbox" 
								id="season-{season.season_id}"
								checked={selectedSeasons.includes(season.season_id)}
								on:change={() => toggleSeasonSelection(season.season_id)}
								class="h-4 w-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
							/>
							<label for="season-{season.season_id}" class="ml-2 block text-sm text-gray-900 cursor-pointer">
								{season.full_title || season.season_title} (ID: {season.season_id})
							</label>
						</div>
					{/each}
				</div>
				<p class="text-xs text-gray-500 mt-1">已选择 {selectedSeasons.length} 个季度</p>
			{/if}
		</div>
		{/if}
		{/if}
		
		<div class="flex justify-end">
			<Button type="submit" disabled={loading}>
				{loading ? '添加中...' : '添加'}
			</Button>
		</div>
	</form>
</div>

<style>
	.line-clamp-1 {
		display: -webkit-box;
		-webkit-line-clamp: 1;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style> 