<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Button } from '$lib/components/ui/button';
	import { toast } from 'svelte-sonner';
	import { goto } from '$app/navigation';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import api from '$lib/api';
	import { onMount, onDestroy } from 'svelte';
	import type { VideoCategory, SearchResultItem } from '$lib/types';
	import { Search, X } from '@lucide/svelte';

	let sourceType: VideoCategory = 'collection';
	let sourceId = '';
	let upId = '';
	let name = '';
	let path = '';
	let collectionType = 'season';
	let downloadAllSeasons = false;
	let loading = false;

	// 添加手动输入标志
	let isManualInput = false;

	// 搜索相关
	let searchKeyword = '';
	let searchLoading = false;
	let searchResults: SearchResultItem[] = [];
	let showSearchResults = false;
	let searchCurrentPage = 1;
	let searchTotalPages = 0;
	let searchTotalResults = 0;

	// 收藏夹相关
	let userFavorites: any[] = [];
	let loadingFavorites = false;

	// UP主合集相关
	let userCollections: any[] = [];
	let loadingCollections = false;
	let upIdTimeout: any;

	// 关注的UP主相关
	let userFollowings: any[] = [];
	let loadingFollowings = false;

	// 番剧季度相关
	let bangumiSeasons: any[] = [];
	let loadingSeasons = false;
	let selectedSeasons: string[] = [];
	let seasonIdTimeout: any;

	// 悬停详情相关
	let hoveredResult: SearchResultItem | null = null;
	let hoverTimeout: any;
	let mousePosition = { x: 0, y: 0 };

	// 响应式相关
	let innerWidth: number;
	let isMobile: boolean = false;
	$: isMobile = innerWidth < 768; // md断点

	// 源类型选项
	const sourceTypeOptions = [
		{ value: 'collection', label: '合集', description: '视频合集，需要UP主ID和合集ID' },
		{ value: 'favorite', label: '收藏夹', description: '收藏夹ID可在收藏夹页面URL中获取' },
		{ value: 'submission', label: 'UP主投稿', description: 'UP主ID可在UP主空间URL中获取' },
		{ value: 'watch_later', label: '稍后观看', description: '同步稍后观看列表' },
		{ value: 'bangumi', label: '番剧', description: '番剧season_id可在番剧页面URL中获取' }
	];

	// 合集类型选项
	const collectionTypeOptions = [
		{ value: 'season', label: '合集', description: 'B站标准合集' },
		{ value: 'series', label: '系列', description: '视频系列' }
	];

	// 订阅的合集相关
	let subscribedCollections: any[] = [];
	let loadingSubscribedCollections = false;

	onMount(() => {
		setBreadcrumb([
			{ label: '主页', href: '/' },
			{ label: '添加视频源', isActive: true }
		]);
	});

	onDestroy(() => {
		// 清理定时器
		clearTimeout(hoverTimeout);
		clearTimeout(upIdTimeout);
		clearTimeout(seasonIdTimeout);
	});

	// 搜索B站内容
	async function handleSearch(page = 1) {
		if (!searchKeyword.trim()) {
			toast.error('请输入搜索关键词');
			return;
		}

		// 根据当前选择的视频源类型确定搜索类型
		let searchType: 'video' | 'bili_user' | 'media_bangumi';
		switch (sourceType) {
			case 'collection':
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

		searchLoading = true;
		
		try {
			const result = await api.searchBilibili({
				keyword: searchKeyword,
				search_type: searchType,
				page: page,
				page_size: 12
			});

			if (result.data.success) {
				searchResults = result.data.results;
				searchCurrentPage = page;
				searchTotalPages = Math.ceil(result.data.total / 12);
				searchTotalResults = result.data.total;
				showSearchResults = true;
			} else {
				toast.error('搜索失败');
			}
		} catch (error: any) {
			console.error('搜索失败:', error);
			toast.error('搜索失败', { description: error.message });
		} finally {
			searchLoading = false;
		}
	}

	// 选择搜索结果
	function selectSearchResult(result: SearchResultItem) {
		switch (sourceType) {
			case 'collection':
				if (result.mid) {
					upId = result.mid.toString();
					// 触发获取UP主合集列表
					handleUpIdChange();
					toast.success('已填充UP主信息', { description: '正在获取合集列表...' });
				}
				break;
			case 'submission':
				if (result.mid) {
					sourceId = result.mid.toString();
					name = cleanTitle(result.title);
				}
				break;
			case 'bangumi':
				if (result.season_id) {
					sourceId = result.season_id;
					name = cleanTitle(result.title);
				}
				break;
			case 'favorite':
			default:
				if (result.bvid) {
					sourceId = result.bvid;
					name = cleanTitle(result.title);
				}
				break;
		}
		
		// 关闭搜索结果
		showSearchResults = false;
		searchResults = [];
		searchKeyword = '';
		searchTotalResults = 0;
		searchCurrentPage = 1;
		searchTotalPages = 0;
		
		// 清除悬停状态
		hoveredResult = null;
		
		if (sourceType !== 'collection') {
			toast.success('已填充信息', { description: '请检查并完善其他必要信息' });
		}
	}

	// 清理标题中的HTML标签
	function cleanTitle(title: string): string {
		const div = document.createElement('div');
		div.innerHTML = title;
		return div.textContent || div.innerText || title;
	}

	// 处理图片URL
	function processBilibiliImageUrl(url: string): string {
		if (!url) return '';
		
		if (url.startsWith('https://')) return url;
		if (url.startsWith('//')) return 'https:' + url;
		if (url.startsWith('http://')) return url.replace('http://', 'https://');
		if (!url.startsWith('http')) return 'https://' + url;
		
		return url.split('@')[0];
	}

	// 处理图片加载错误
	function handleImageError(event: Event) {
		const img = event.target as HTMLImageElement;
		// 使用默认的占位图片
		img.style.display = 'none';
		const parent = img.parentElement;
		if (parent && !parent.querySelector('.placeholder')) {
			const placeholder = document.createElement('div');
			// 获取原图片的尺寸类
			const widthClass = img.className.match(/w-\d+/)?.[0] || 'w-20';
			const heightClass = img.className.match(/h-\d+/)?.[0] || 'h-14';
			placeholder.className = `placeholder ${widthClass} ${heightClass} bg-gray-200 rounded flex items-center justify-center text-xs text-gray-400`;
			placeholder.textContent = '无图片';
			parent.appendChild(placeholder);
		}
	}

	async function handleSubmit() {
		// 验证表单
		if (sourceType !== 'watch_later' && !sourceId) {
			toast.error('请输入ID', { description: '视频源ID不能为空' });
			return;
		}

		if (sourceType === 'collection' && !upId) {
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

		// 番剧特殊验证
		if (sourceType === 'bangumi') {
			if (!downloadAllSeasons && selectedSeasons.length === 0) {
				toast.error('请选择要下载的季度', { 
					description: '未选择"下载全部季度"时，至少需要选择一个季度' 
				});
				return;
			}
		}

		loading = true;

		try {
			const params: any = {
				source_type: sourceType,
				source_id: sourceId,
				name,
				path
			};

			if (sourceType === 'collection') {
				params.up_id = upId;
				params.collection_type = collectionType;
			}

			if (sourceType === 'bangumi') {
				params.download_all_seasons = downloadAllSeasons;
				// 如果选择了特定季度，添加selected_seasons参数
				if (selectedSeasons.length > 0 && !downloadAllSeasons) {
					params.selected_seasons = selectedSeasons;
				}
			}

			const result = await api.addVideoSource(params);

			if (result.data.success) {
				toast.success('添加成功', { description: result.data.message });
				// 重置表单
				sourceId = '';
				upId = '';
				name = '';
				path = '/Downloads';
				downloadAllSeasons = false;
				collectionType = 'season';
				isManualInput = false;
				bangumiSeasons = [];
				selectedSeasons = [];
				// 跳转到首页
				goto('/');
			} else {
				toast.error('添加失败', { description: result.data.message });
			}
		} catch (error: any) {
			console.error('添加视频源失败:', error);
			
			// 解析错误信息，提供更友好的提示
			let errorMessage = error.message;
			let errorDescription = '';

			if (errorMessage.includes('已存在')) {
				// 重复添加错误
				if (sourceType === 'bangumi') {
					errorDescription = '该番剧已经添加过了，请检查是否使用了相同的Season ID、Media ID或Episode ID';
				} else if (sourceType === 'collection') {
					errorDescription = '该合集已经添加过了，请检查是否使用了相同的合集ID和UP主ID';
				} else if (sourceType === 'favorite') {
					errorDescription = '该收藏夹已经添加过了，请检查是否使用了相同的收藏夹ID';
				} else if (sourceType === 'submission') {
					errorDescription = '该UP主的投稿已经添加过了，请检查是否使用了相同的UP主ID';
				} else if (sourceType === 'watch_later') {
					errorDescription = '稍后观看只能配置一个，请先删除现有配置';
				}
				
				toast.error('重复添加', { 
					description: errorDescription,
					duration: 5000 // 延长显示时间
				});
			} else {
				// 其他错误
				toast.error('添加失败', { description: errorMessage });
			}
		} finally {
			loading = false;
		}
	}

	// 根据类型显示不同的描述
	$: currentTypeDescription = sourceTypeOptions.find(opt => opt.value === sourceType)?.description || '';

	// 获取收藏夹列表
	async function fetchUserFavorites() {
		loadingFavorites = true;
		try {
			const result = await api.getUserFavorites();
			if (result.data) {
			userFavorites = result.data;
				toast.success('获取收藏夹成功', { description: `共获取到 ${userFavorites.length} 个收藏夹` });
			} else {
				toast.error('获取收藏夹失败');
			}
		} catch (error: any) {
			console.error('获取收藏夹失败:', error);
			toast.error('获取收藏夹失败', { description: error.message });
		} finally {
			loadingFavorites = false;
		}
	}

	// 选择收藏夹
	function selectFavorite(favorite: any) {
		sourceId = favorite.id.toString();
		name = favorite.name || favorite.title;
		toast.success('已选择收藏夹', { description: name });
	}

	// 处理UP主ID变化
	function handleUpIdChange() {
		clearTimeout(upIdTimeout);
		if (upId.trim()) {
			upIdTimeout = setTimeout(() => {
				fetchUserCollections();
			}, 500);
		} else {
			userCollections = [];
		}
	}

	// 获取UP主合集列表
	async function fetchUserCollections() {
		if (!upId.trim()) return;
		
		loadingCollections = true;
		try {
			const result = await api.getUserCollections(upId);
			if (result.data && result.data.collections) {
				userCollections = result.data.collections;
				if (userCollections.length === 0) {
					toast.info('该UP主暂无合集');
				} else {
					toast.success('获取合集列表成功', { description: `共获取到 ${userCollections.length} 个合集` });
				}
			} else {
				toast.error('获取合集列表失败');
				userCollections = [];
			}
		} catch (error: any) {
			console.error('获取合集列表失败:', error);
			toast.error('获取合集列表失败', { description: error.message });
			userCollections = [];
		} finally {
			loadingCollections = false;
		}
	}

	// 选择合集
	function selectCollection(collection: any) {
		sourceId = collection.sid;
		name = collection.name;
		collectionType = collection.collection_type;
		isManualInput = false;  // 从列表选择，不是手动输入
		toast.success('已选择合集', { description: `${collection.collection_type === 'season' ? '合集' : '系列'}：${collection.name}` });
	}

	// 处理Season ID变化
	function handleSeasonIdChange() {
		clearTimeout(seasonIdTimeout);
		if (sourceId.trim() && sourceType === 'bangumi') {
			seasonIdTimeout = setTimeout(() => {
				fetchBangumiSeasons();
			}, 500);
		} else {
			bangumiSeasons = [];
			selectedSeasons = [];
		}
	}

	// 获取番剧季度信息
	async function fetchBangumiSeasons() {
		if (!sourceId.trim() || sourceType !== 'bangumi') return;
		
		loadingSeasons = true;
		try {
			const result = await api.getBangumiSeasons(sourceId);
			if (result.data && result.data.success) {
				bangumiSeasons = result.data.data || [];
				// 默认选中当前季度
				if (bangumiSeasons.length > 0) {
					const currentSeason = bangumiSeasons.find(s => s.season_id === sourceId);
					if (currentSeason) {
						selectedSeasons = [currentSeason.season_id];
					}
				}
			} else {
				bangumiSeasons = [];
			}
		} catch (error: any) {
			console.error('获取季度信息失败:', error);
			toast.error('获取季度信息失败', { description: error.message });
			bangumiSeasons = [];
			selectedSeasons = [];
		} finally {
			loadingSeasons = false;
		}
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

	// 监听sourceType变化，清理季度相关状态
	$: if (sourceType !== 'bangumi') {
		bangumiSeasons = [];
		selectedSeasons = [];
	}
	
	// 监听sourceType变化，重置手动输入标志
	$: if (sourceType) {
		isManualInput = false;
	}
	
	// 监听 source_id 变化，自动获取季度信息
	$: if (sourceType === 'bangumi' && sourceId) {
		fetchBangumiSeasons();
	}

	// 处理鼠标悬停
	function handleMouseEnter(result: SearchResultItem, event: MouseEvent) {
		hoveredResult = result;
		updateTooltipPosition(event);
	}

	function handleMouseMove(event: MouseEvent) {
		if (hoveredResult) {
			updateTooltipPosition(event);
		}
	}
	
	function updateTooltipPosition(event: MouseEvent) {
		const x = event.pageX + 20;
		const y = event.pageY - 100;
		
		mousePosition = { 
			x: Math.max(10, x), 
			y: Math.max(10, y) 
		};
	}

	function handleMouseLeave() {
		hoveredResult = null;
	}

	// 获取关注的UP主列表
	async function fetchUserFollowings() {
		loadingFollowings = true;
		try {
			const result = await api.getUserFollowings();
			if (result.data) {
				userFollowings = result.data;
				toast.success('获取关注UP主成功', { description: `共获取到 ${userFollowings.length} 个UP主` });
			} else {
				toast.error('获取关注UP主失败');
			}
		} catch (error: any) {
			console.error('获取关注UP主失败:', error);
			toast.error('获取关注UP主失败', { description: error.message });
		} finally {
			loadingFollowings = false;
		}
	}

	// 选择关注的UP主
	function selectFollowing(following: any) {
		switch (sourceType) {
			case 'collection':
				upId = following.mid.toString();
				// 触发获取UP主合集列表
				handleUpIdChange();
				toast.success('已填充UP主信息', { description: '正在获取合集列表...' });
				break;
			case 'submission':
				sourceId = following.mid.toString();
				name = following.name;
				toast.success('已填充UP主信息');
				break;
		}
	}

	// 获取关注的收藏夹列表  
	async function fetchSubscribedCollections() {
		loadingSubscribedCollections = true;
		try {
			const result = await api.getSubscribedCollections();
			if (result.data) {
				subscribedCollections = result.data;
				if (subscribedCollections.length === 0) {
					toast.info('暂无关注的合集', { 
						description: '您还没有关注任何合集。关注合集后可以在这里快速选择添加。',
						duration: 5000
					});
				} else {
					toast.success('获取关注的合集成功', { description: `共获取到 ${subscribedCollections.length} 个您关注的合集` });
				}
			} else {
				toast.error('获取合集失败');
			}
		} catch (error: any) {
			console.error('获取合集失败:', error);
			toast.error('获取合集失败', { description: error.message });
		} finally {
			loadingSubscribedCollections = false;
		}
	}

	// 选择订阅的合集
	function selectSubscribedCollection(collection: any) {
		sourceId = collection.sid;
		name = collection.name;
		upId = collection.up_mid.toString();
		collectionType = collection.collection_type;
		toast.success('已选择订阅合集', { description: collection.name });
	}
</script>

<svelte:head>
	<title>添加视频源 - Bili Sync</title>
</svelte:head>

<svelte:window bind:innerWidth />

<div class="py-2">
	<div class="mx-auto px-4">
		<div class="bg-card rounded-lg shadow-sm border p-6">
			<h1 class="text-2xl font-bold mb-6">添加新视频源</h1>

			<div class="flex {isMobile ? 'flex-col' : 'gap-8'}">
				<!-- 左侧：表单区域 -->
				<div class="{isMobile ? 'w-full' : 'w-[600px] flex-shrink-0'}">
					<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="space-y-6">
						<!-- 视频源类型 -->
						<div class="space-y-2">
							<Label for="source-type">视频源类型</Label>
							<select 
								id="source-type"
								bind:value={sourceType}
								class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
							>
								{#each sourceTypeOptions as option}
									<option value={option.value}>{option.label}</option>
								{/each}
							</select>
							<p class="text-sm text-muted-foreground">{currentTypeDescription}</p>
						</div>

						<!-- 搜索功能 -->
						{#if sourceType !== 'favorite' && sourceType !== 'watch_later'}
							<div class="p-4 bg-blue-50 rounded-lg border border-blue-200">
								<div class="space-y-2">
									<div>
										<Label for="search">
											{#if sourceType === 'collection'}
												搜索UP主
											{:else if sourceType === 'submission'}
												搜索UP主
											{:else if sourceType === 'bangumi'}
												搜索番剧
											{:else}
												搜索B站内容
											{/if}
										</Label>
										<div class="flex {isMobile ? 'flex-col gap-2' : 'gap-2'} mt-2">
											<Input 
												id="search"
												bind:value={searchKeyword} 
												placeholder={sourceType === 'submission' || sourceType === 'collection' ? '搜索UP主...' : sourceType === 'bangumi' ? '搜索番剧...' : '搜索视频...'}
												onkeydown={(e) => e.key === 'Enter' && handleSearch()}
											/>
											<div class="flex gap-2">
											<Button 
												onclick={() => handleSearch()} 
												disabled={searchLoading || !searchKeyword.trim()}
												size="sm"
													class="{isMobile ? 'flex-1' : ''}"
											>
												{#if searchLoading}
													搜索中...
												{:else}
													<Search class="h-4 w-4" />
												{/if}
											</Button>
											{#if sourceType === 'collection' || sourceType === 'submission'}
												<Button 
													onclick={sourceType === 'collection' ? fetchSubscribedCollections : fetchUserFollowings} 
													disabled={sourceType === 'collection' ? loadingSubscribedCollections : loadingFollowings}
													size="sm"
													variant="outline"
														class="{isMobile ? 'flex-1' : ''}"
												>
													{sourceType === 'collection' 
														? (loadingSubscribedCollections ? '获取中...' : '获取关注的合集')
														: (loadingFollowings ? '获取中...' : '获取关注')
													}
												</Button>
											{/if}
											</div>
										</div>
										<p class="text-xs text-gray-600 mt-1">
											{#if sourceType === 'collection'}
												搜索UP主后会自动填充UP主ID，并显示该UP主的所有合集供选择
											{:else if sourceType === 'submission'}
												搜索并选择UP主，将自动填充UP主ID
											{:else if sourceType === 'bangumi'}
												搜索并选择番剧，将自动填充Season ID
											{:else}
												根据当前选择的视频源类型搜索对应内容
											{/if}
										</p>
									</div>
								</div>
							</div>
						{/if}

						<!-- 收藏夹列表（仅收藏夹类型时显示） -->
						{#if sourceType === 'favorite'}
							<div class="p-4 bg-yellow-50 rounded-lg border border-yellow-200">
								<div class="flex {isMobile ? 'flex-col gap-2' : 'items-center justify-between'} mb-2">
									<span class="text-sm font-medium text-yellow-800">我的收藏夹</span>
									<Button 
										size="sm" 
										variant="outline"
										onclick={fetchUserFavorites}
										disabled={loadingFavorites}
										class="{isMobile ? 'w-full' : ''}"
									>
										{loadingFavorites ? '加载中...' : '获取收藏夹'}
									</Button>
								</div>
								
								{#if userFavorites.length > 0}
									<p class="text-xs text-yellow-600">已获取 {userFavorites.length} 个收藏夹，请在{isMobile ? '下方' : '右侧'}选择</p>
								{/if}
							</div>
						{/if}

						<!-- 合集类型（仅合集时显示，且手动输入） -->
						{#if sourceType === 'collection' && isManualInput}
							<div class="space-y-2">
								<Label for="collection-type">合集类型</Label>
								<select 
									id="collection-type"
									bind:value={collectionType}
									class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
								>
									{#each collectionTypeOptions as option}
										<option value={option.value}>{option.label}</option>
									{/each}
								</select>
								<p class="text-sm text-orange-600">⚠️ 手动输入合集ID时需要指定类型，建议从{isMobile ? '下方' : '右侧'}合集列表中选择</p>
							</div>
						{/if}

						<!-- UP主ID（仅合集时显示） -->
						{#if sourceType === 'collection'}
							<div class="space-y-2">
								<Label for="up-id">UP主ID</Label>
								<Input 
									id="up-id" 
									bind:value={upId} 
									placeholder="请输入UP主ID"
									onblur={handleUpIdChange}
									required
								/>
								{#if userCollections.length > 0}
									<p class="text-xs text-green-600 mt-1">✓ 已获取合集列表，请在{isMobile ? '下方' : '右侧'}选择</p>
								{/if}
							</div>
						{/if}

						<!-- 视频源ID（稍后观看除外） -->
						{#if sourceType !== 'watch_later'}
							<div class="space-y-2">
								<Label for="source-id">
									{#if sourceType === 'collection'}合集ID
									{:else if sourceType === 'favorite'}收藏夹ID
									{:else if sourceType === 'submission'}UP主ID
									{:else if sourceType === 'bangumi'}Season ID
									{:else}ID{/if}
								</Label>
								<Input 
									id="source-id" 
									bind:value={sourceId} 
									placeholder={`请输入${sourceType === 'collection' ? '合集' : sourceType === 'favorite' ? '收藏夹' : sourceType === 'submission' ? 'UP主' : sourceType === 'bangumi' ? 'Season' : ''}ID`}
									oninput={() => {
										if (sourceType === 'collection') {
											isManualInput = true;
										}
									}}
									required
								/>
								{#if sourceType === 'collection' && !isManualInput && sourceId}
									<p class="text-xs text-green-600 mt-1">✓ 已从列表中选择合集，类型已自动识别</p>
								{/if}
								{#if sourceType === 'favorite' && sourceId}
									<p class="text-xs text-green-600 mt-1">✓ 已选择收藏夹</p>
								{/if}
								
								<!-- 下载所有季度（仅番剧时显示，紧跟在Season ID后面） -->
								{#if sourceType === 'bangumi' && sourceId && bangumiSeasons.length > 0 && !loadingSeasons}
									<div class="flex items-center space-x-2 mt-3">
										<input 
											type="checkbox"
											id="download-all-seasons" 
											bind:checked={downloadAllSeasons}
											onchange={() => { if (downloadAllSeasons) selectedSeasons = []; }}
											class="h-4 w-4 rounded border-gray-300 text-primary focus:ring-primary"
										/>
										<Label 
											for="download-all-seasons" 
											class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
										>
											下载所有季度
										</Label>
									</div>
									{#if downloadAllSeasons}
										<p class="text-xs text-purple-600 mt-1 ml-6">勾选后将下载该番剧的所有季度，无需单独选择</p>
									{:else if bangumiSeasons.length > 1}
										<p class="text-xs text-purple-600 mt-1 ml-6">检测到 {bangumiSeasons.length} 个相关季度，请在{isMobile ? '下方' : '右侧'}选择要下载的季度</p>
									{:else if bangumiSeasons.length === 1}
										<p class="text-xs text-purple-600 mt-1 ml-6">该番剧只有当前一个季度</p>
									{/if}
								{:else if sourceType === 'bangumi' && sourceId && loadingSeasons}
									<p class="text-xs text-purple-600 mt-3">正在获取季度信息...</p>
								{/if}
							</div>
						{/if}

						<!-- 名称 -->
						<div class="space-y-2">
							<Label for="name">名称</Label>
							<Input 
								id="name" 
								bind:value={name} 
								placeholder="请输入视频源名称"
								required
							/>
						</div>

						<!-- 保存路径 -->
						<div class="space-y-2">
							<Label for="path">保存路径</Label>
							<Input 
								id="path" 
								bind:value={path} 
								placeholder="例如：D:/Videos/Bilibili"
								required
							/>
							<p class="text-sm text-muted-foreground">请输入绝对路径</p>
						</div>

						<!-- 提交按钮 -->
						<div class="flex {isMobile ? 'flex-col' : ''} gap-2">
							<Button type="submit" disabled={loading} class="{isMobile ? 'w-full' : ''}">
								{loading ? '添加中...' : '添加'}
							</Button>
							<Button type="button" variant="outline" onclick={() => goto('/')} class="{isMobile ? 'w-full' : ''}">
								取消
							</Button>
						</div>
					</form>
				</div>

				<!-- 右侧：搜索结果区域 -->
				{#if showSearchResults && searchResults.length > 0}
					<div class="{isMobile ? 'w-full mt-6' : 'flex-1'}">
						<div class="bg-white rounded-lg border {isMobile ? '' : 'h-full'} overflow-hidden flex flex-col {isMobile ? '' : 'sticky top-6'} max-h-[calc(100vh-200px)]">
							<div class="flex justify-between items-center p-4 border-b bg-gray-50">
								<div>
									<span class="text-base font-medium">搜索结果</span>
									<span class="text-sm text-gray-600 {isMobile ? 'block' : 'ml-2'}">
										共找到 {searchTotalResults} 个结果，当前第 {searchCurrentPage} 页
									</span>
								</div>
								<button 
									onclick={() => { 
										showSearchResults = false; 
										searchResults = []; 
										searchTotalResults = 0;
										searchCurrentPage = 1;
										searchTotalPages = 0;
									}}
									class="text-gray-500 hover:text-gray-700 text-xl p-1"
								>
									<X class="h-5 w-5" />
								</button>
							</div>
							
							<div class="flex-1 overflow-y-auto p-3">
								<div class="grid {isMobile ? 'grid-cols-1 gap-3' : 'grid-cols-3 gap-4'}">
									{#each searchResults as result}
										<button 
											onclick={() => selectSearchResult(result)}
											onmouseenter={(e) => handleMouseEnter(result, e)}
											onmouseleave={handleMouseLeave}
											onmousemove={handleMouseMove}
											class="p-4 hover:bg-gray-50 border rounded-lg flex gap-3 text-left transition-colors items-start relative"
										>
											{#if result.cover}
												<img 
													src={processBilibiliImageUrl(result.cover)}
													alt={result.title}
													class="{sourceType === 'bangumi' ? 'w-14 h-20' : 'w-20 h-14'} object-cover rounded flex-shrink-0"
													onerror={handleImageError}
													loading="lazy"
													crossorigin="anonymous"
													referrerpolicy="no-referrer"
												/>
											{:else}
												<div class="{sourceType === 'bangumi' ? 'w-14 h-20' : 'w-20 h-14'} bg-gray-200 rounded flex items-center justify-center text-xs text-gray-600 flex-shrink-0">
													无图片
												</div>
											{/if}
											<div class="flex-1 min-w-0">
												<h4 class="font-medium text-sm truncate">{@html result.title}</h4>
												<p class="text-xs text-gray-600 truncate mt-1">{result.author}</p>
												{#if result.description}
													<p class="text-xs text-gray-500 line-clamp-2 mt-1">{result.description}</p>
												{/if}
											</div>
										</button>
									{/each}
								</div>
							</div>

							{#if searchTotalPages > 1}
								<div class="flex justify-center items-center gap-2 p-3 border-t">
									<Button 
										size="sm" 
										variant="outline"
										onclick={() => handleSearch(searchCurrentPage - 1)}
										disabled={searchCurrentPage <= 1 || searchLoading}
									>
										上一页
									</Button>
									<span class="text-sm text-gray-600">
										{searchCurrentPage} / {searchTotalPages}
									</span>
									<Button 
										size="sm" 
										variant="outline"
										onclick={() => handleSearch(searchCurrentPage + 1)}
										disabled={searchCurrentPage >= searchTotalPages || searchLoading}
									>
										下一页
									</Button>
								</div>
							{/if}
						</div>
					</div>
				{/if}

				<!-- 关注UP主列表（移动到右侧） -->
				{#if (sourceType === 'collection' || sourceType === 'submission') && userFollowings.length > 0}
					<div class="{isMobile ? 'w-full mt-6' : 'flex-1'}">
						<div class="bg-white rounded-lg border {isMobile ? '' : 'h-full'} overflow-hidden flex flex-col {isMobile ? '' : 'sticky top-6'} max-h-126">
							<div class="flex justify-between items-center p-4 border-b bg-blue-50">
								<div>
									<span class="text-base font-medium text-blue-800">关注的UP主</span>
									<span class="text-sm text-blue-600 {isMobile ? 'block' : 'ml-2'}">
										共 {userFollowings.length} 个UP主
									</span>
								</div>
							</div>
							
							<div class="flex-1 overflow-y-auto p-3">
								<div class="grid {isMobile ? 'grid-cols-1 gap-2' : 'grid-cols-3 gap-3'}">
									{#each userFollowings as following}
										<button 
											onclick={() => selectFollowing(following)}
											class="p-3 hover:bg-gray-50 border rounded-lg text-left transition-colors"
										>
											<div class="flex items-start gap-2">
												{#if following.face}
													<img 
														src={processBilibiliImageUrl(following.face)}
														alt={following.name}
														class="w-10 h-10 object-cover rounded-full flex-shrink-0"
														onerror={handleImageError}
														loading="lazy"
														crossorigin="anonymous"
														referrerpolicy="no-referrer"
													/>
												{:else}
													<div class="w-10 h-10 bg-gray-200 rounded-full flex items-center justify-center text-xs text-gray-400 flex-shrink-0">
														头像
													</div>
												{/if}
												<div class="flex-1 min-w-0">
													<div class="flex items-center gap-1 mb-1">
														<h4 class="font-medium text-xs truncate">{following.name}</h4>
														{#if following.official_verify && following.official_verify.type >= 0}
															<span class="text-xs px-1 py-0.5 rounded flex-shrink-0 bg-yellow-100 text-yellow-700">
																V
															</span>
														{/if}
													</div>
													<p class="text-xs text-gray-600 mb-1 truncate">UID: {following.mid}</p>
													{#if following.sign}
														<p class="text-xs text-gray-500 line-clamp-1">{following.sign}</p>
													{/if}
												</div>
											</div>
										</button>
									{/each}
								</div>
							</div>
						</div>
					</div>
				{/if}

				<!-- UP主合集列表（移动到右侧） -->
				{#if sourceType === 'collection' && userCollections.length > 0}
					<div class="{isMobile ? 'w-full mt-6' : 'flex-1'}">
						<div class="bg-white rounded-lg border {isMobile ? '' : 'h-full'} overflow-hidden flex flex-col {isMobile ? '' : 'sticky top-6'} max-h-[calc(100vh-200px)]">
							<div class="flex justify-between items-center p-4 border-b bg-green-50">
								<div>
									<span class="text-base font-medium text-green-800">UP主合集列表</span>
									<span class="text-sm text-green-600 {isMobile ? 'block' : 'ml-2'}">
										共 {userCollections.length} 个合集
									</span>
								</div>
							</div>
							
							<div class="flex-1 overflow-y-auto p-3">
								<div class="grid {isMobile ? 'grid-cols-1 gap-3' : 'grid-cols-2 gap-4'}">
									{#each userCollections as collection}
										<button 
											onclick={() => selectCollection(collection)}
											class="p-4 hover:bg-gray-50 border rounded-lg text-left transition-colors"
										>
											<div class="flex items-start gap-3">
												{#if collection.cover}
													<img 
														src={processBilibiliImageUrl(collection.cover)}
														alt={collection.name}
														class="w-24 h-16 object-cover rounded flex-shrink-0"
														onerror={handleImageError}
														loading="lazy"
														crossorigin="anonymous"
														referrerpolicy="no-referrer"
													/>
												{:else}
													<div class="w-24 h-16 bg-gray-200 rounded flex items-center justify-center text-xs text-gray-400 flex-shrink-0">
														无封面
													</div>
												{/if}
												<div class="flex-1 min-w-0">
													<div class="flex items-center gap-2 mb-1">
														<h4 class="font-medium text-sm truncate">{collection.name}</h4>
														<span class="text-xs px-2 py-0.5 rounded flex-shrink-0 {collection.collection_type === 'season' ? 'bg-green-100 text-green-700' : 'bg-blue-100 text-blue-700'}">
															{collection.collection_type === 'season' ? '合集' : '系列'}
														</span>
													</div>
													<p class="text-xs text-gray-600 mb-1">ID: {collection.sid}</p>
													<p class="text-xs text-gray-600">共 {collection.total} 个视频</p>
													{#if collection.description}
														<p class="text-xs text-gray-500 line-clamp-2 mt-1">{collection.description}</p>
													{/if}
												</div>
											</div>
										</button>
									{/each}
								</div>
							</div>
						</div>
					</div>
				{/if}

				<!-- 收藏夹列表（移动到右侧） -->
				{#if sourceType === 'favorite' && userFavorites.length > 0}
					<div class="{isMobile ? 'w-full mt-6' : 'flex-1'}">
						<div class="bg-white rounded-lg border {isMobile ? '' : 'h-full'} overflow-hidden flex flex-col {isMobile ? '' : 'sticky top-6'} max-h-[calc(100vh-200px)]">
							<div class="flex justify-between items-center p-4 border-b bg-yellow-50">
								<div>
									<span class="text-base font-medium text-yellow-800">我的收藏夹</span>
									<span class="text-sm text-yellow-600 {isMobile ? 'block' : 'ml-2'}">
										共 {userFavorites.length} 个收藏夹
									</span>
								</div>
							</div>
							
							<div class="flex-1 overflow-y-auto p-3">
								<div class="grid {isMobile ? 'grid-cols-1 gap-3' : 'grid-cols-2 gap-4'}">
									{#each userFavorites as favorite}
										<button 
											onclick={() => selectFavorite(favorite)}
											class="p-4 hover:bg-gray-50 border rounded-lg text-left transition-colors"
										>
											<div class="flex items-start gap-3">
												{#if favorite.cover}
													<img 
														src={processBilibiliImageUrl(favorite.cover)}
														alt={favorite.name || favorite.title}
														class="w-24 h-16 object-cover rounded flex-shrink-0"
														onerror={handleImageError}
														loading="lazy"
														crossorigin="anonymous"
														referrerpolicy="no-referrer"
													/>
												{:else}
													<div class="w-24 h-16 bg-gray-200 rounded flex items-center justify-center text-xs text-gray-400 flex-shrink-0">
														无封面
													</div>
												{/if}
												<div class="flex-1 min-w-0">
													<h4 class="font-medium text-sm truncate mb-1">{favorite.name || favorite.title}</h4>
													<p class="text-xs text-gray-600 mb-1">收藏夹ID: {favorite.id}</p>
													<p class="text-xs text-gray-600 mb-1">共 {favorite.media_count} 个视频</p>
													{#if favorite.created}
														<p class="text-xs text-gray-500">创建于 {new Date(favorite.created * 1000).toLocaleDateString()}</p>
													{/if}
												</div>
											</div>
										</button>
									{/each}
								</div>
							</div>
						</div>
					</div>
				{/if}

				<!-- 番剧季度选择区域（移动到右侧） -->
				{#if sourceType === 'bangumi' && sourceId && !downloadAllSeasons && bangumiSeasons.length > 1}
					<div class="{isMobile ? 'w-full mt-6' : 'flex-1'}">
						<div class="bg-white rounded-lg border {isMobile ? '' : 'h-full'} overflow-hidden flex flex-col {isMobile ? '' : 'sticky top-6'} max-h-[calc(100vh-200px)]">
							<div class="flex justify-between items-center p-4 border-b bg-purple-50">
								<div>
									<span class="text-base font-medium text-purple-800">选择要下载的季度</span>
									<span class="text-sm text-purple-600 {isMobile ? 'block' : 'ml-2'}">
										{#if loadingSeasons}
											正在加载...
										{:else if bangumiSeasons.length > 0}
											共 {bangumiSeasons.length} 个相关季度
										{:else}
											暂无季度信息
										{/if}
									</span>
								</div>
								{#if selectedSeasons.length > 0}
									<span class="text-xs text-purple-700 bg-purple-100 px-2 py-1 rounded">
										已选择 {selectedSeasons.length} 个
										{#if selectedSeasons.length === bangumiSeasons.length}
											（全部）
										{/if}
									</span>
								{/if}
							</div>
							
							<div class="flex-1 overflow-hidden p-3">
								{#if loadingSeasons}
									<div class="p-4 text-center">
										<div class="text-sm text-purple-700">正在加载季度信息...</div>
									</div>
								{:else if bangumiSeasons.length > 0}
									<div class="seasons-grid-container">
										<div class="grid {isMobile ? 'grid-cols-1 gap-3' : 'grid-cols-3 gap-4'}">
											{#each bangumiSeasons as season}
												<div class="p-4 border rounded-lg hover:bg-purple-50 transition-colors relative {isMobile ? 'h-auto' : 'h-[120px]'}">
													<div class="flex gap-3 {isMobile ? '' : 'h-full'}">
														{#if season.cover}
															<img 
																src={processBilibiliImageUrl(season.cover)}
																alt={season.season_title || season.title}
																class="w-14 h-20 object-cover rounded flex-shrink-0"
																onerror={handleImageError}
																loading="lazy"
																crossorigin="anonymous"
																referrerpolicy="no-referrer"
															/>
														{:else}
															<div class="w-14 h-20 bg-gray-200 rounded flex items-center justify-center text-xs text-gray-400 flex-shrink-0">
																无封面
															</div>
														{/if}
														<div class="flex-1 min-w-0">
															<div class="absolute top-3 right-3">
																<input 
																	type="checkbox"
																	id="season-{season.season_id}"
																	checked={selectedSeasons.includes(season.season_id)}
																	onchange={() => toggleSeasonSelection(season.season_id)}
																	class="h-4 w-4 rounded border-gray-300 text-purple-600 focus:ring-purple-500"
																/>
															</div>
															<!-- 右下角集数标签 -->
															{#if season.episode_count}
																<div class="absolute bottom-3 right-3">
																	<span class="text-xs bg-purple-100 text-purple-700 px-1.5 py-0.5 rounded">{season.episode_count}集</span>
																</div>
															{/if}
															<label for="season-{season.season_id}" class="cursor-pointer">
																<h4 class="font-medium text-sm truncate pr-6">{season.full_title || season.season_title || season.title}</h4>
																{#if season.season_id === sourceId}
																	<span class="text-xs bg-purple-100 text-purple-700 px-1.5 py-0.5 rounded inline-block mt-1">当前</span>
																{/if}
																<p class="text-xs text-gray-600 mt-1">Season ID: {season.season_id}</p>
																{#if season.media_id}
																	<p class="text-xs text-gray-500">Media ID: {season.media_id}</p>
																{/if}
															</label>
														</div>
													</div>
												</div>
											{/each}
										</div>
									</div>
									{#if !loadingSeasons && bangumiSeasons.length > 0}
										<p class="text-xs text-purple-600 mt-3 text-center">不选择则仅下载{isMobile ? '上方' : '左侧'}输入的当前季度</p>
									{/if}
								{:else if sourceId}
									<div class="p-4 text-center">
										<div class="text-sm text-gray-500">暂无季度信息</div>
										<div class="text-xs text-gray-400 mt-1">请检查Season ID是否正确</div>
									</div>
								{/if}
							</div>
						</div>
					</div>
				{/if}

				<!-- 订阅的合集列表（仅合集类型时显示） -->
				{#if sourceType === 'collection' && subscribedCollections.length > 0}
					<div class="{isMobile ? 'w-full mt-6' : 'flex-1'}">
						<div class="bg-white rounded-lg border {isMobile ? '' : 'h-full'} overflow-hidden flex flex-col {isMobile ? '' : 'sticky top-6'} max-h-96">
							<div class="flex justify-between items-center p-4 border-b bg-purple-50">
								<div>
									<span class="text-base font-medium text-purple-800">关注的合集</span>
									<span class="text-sm text-purple-600 {isMobile ? 'block' : 'ml-2'}">
										共 {subscribedCollections.length} 个合集
									</span>
			</div>
		</div>
							
							<div class="flex-1 overflow-y-auto p-3">
								<div class="grid {isMobile ? 'grid-cols-1 gap-3' : 'grid-cols-2 gap-4'}">
									{#each subscribedCollections as collection}
										<button 
											onclick={() => selectSubscribedCollection(collection)}
											class="p-4 hover:bg-gray-50 border rounded-lg text-left transition-colors"
										>
											<div class="flex items-start gap-3">
												{#if collection.cover}
													<img 
														src={processBilibiliImageUrl(collection.cover)}
														alt={collection.name}
														class="w-24 h-16 object-cover rounded flex-shrink-0"
														onerror={handleImageError}
														loading="lazy"
														crossorigin="anonymous"
														referrerpolicy="no-referrer"
													/>
												{:else}
													<div class="w-24 h-16 bg-gray-200 rounded flex items-center justify-center text-xs text-gray-400 flex-shrink-0">
														无封面
													</div>
												{/if}
												<div class="flex-1 min-w-0">
													<div class="flex items-center gap-2 mb-1">
														<h4 class="font-medium text-sm truncate">{collection.name}</h4>
														<span class="text-xs px-2 py-0.5 rounded flex-shrink-0 bg-purple-100 text-purple-700">
															{collection.collection_type === 'season' ? '合集' : '系列'}
														</span>
													</div>
													<p class="text-xs text-gray-600 mb-1">ID: {collection.sid}</p>
													<p class="text-xs text-gray-600 mb-1">UP主: {collection.up_name}</p>
													<p class="text-xs text-gray-600">共 {collection.total} 个视频</p>
													{#if collection.description}
														<p class="text-xs text-gray-500 line-clamp-2 mt-1">{collection.description}</p>
													{/if}
												</div>
											</div>
										</button>
									{/each}
								</div>
							</div>
						</div>
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>

<!-- 悬停详情框 -->
{#if hoveredResult}
	<div 
		class="fixed z-50 bg-white rounded-lg shadow-2xl border p-4 max-w-md pointer-events-none"
		style="left: {mousePosition.x}px; top: {mousePosition.y}px;"
	>
		<div class="flex gap-4">
			{#if hoveredResult.cover}
				<img 
					src={processBilibiliImageUrl(hoveredResult.cover)}
					alt={hoveredResult.title}
					class="{sourceType === 'bangumi' ? 'w-24 h-32' : 'w-32 h-20'} object-cover rounded flex-shrink-0"
					loading="lazy"
					crossorigin="anonymous"
					referrerpolicy="no-referrer"
				/>
			{:else}
				<div class="{sourceType === 'bangumi' ? 'w-24 h-32' : 'w-32 h-20'} bg-gray-200 rounded flex items-center justify-center text-sm text-gray-400 flex-shrink-0">
					无图片
				</div>
			{/if}
			<div class="flex-1 min-w-0">
				<h4 class="font-semibold text-sm mb-1">{@html hoveredResult.title}</h4>
				<p class="text-xs text-gray-600 mb-2">作者：{hoveredResult.author}</p>
				{#if hoveredResult.description}
					<p class="text-xs text-gray-500 line-clamp-4 mb-2">{hoveredResult.description}</p>
				{/if}
				<div class="flex flex-wrap gap-2 text-xs">
					{#if hoveredResult.play}
						<span class="flex items-center gap-1 text-gray-500">
							<span>▶</span> 播放：{hoveredResult.play > 10000 ? (hoveredResult.play / 10000).toFixed(1) + '万' : hoveredResult.play}
						</span>
					{/if}
					{#if hoveredResult.danmaku}
						<span class="flex items-center gap-1 text-gray-500">
							<span>💬</span> 弹幕：{hoveredResult.danmaku > 10000 ? (hoveredResult.danmaku / 10000).toFixed(1) + '万' : hoveredResult.danmaku}
						</span>
					{/if}
					{#if sourceType === 'bangumi' && hoveredResult.season_id}
						<span class="text-gray-500">Season ID: {hoveredResult.season_id}</span>
					{/if}
					{#if hoveredResult.bvid}
						<span class="text-gray-500">BV号: {hoveredResult.bvid}</span>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	/* 确保图片加载失败时的占位符正确显示 */
	:global(.placeholder) {
		flex-shrink: 0;
	}
	
	/* 限制描述文字的行数 */
	.line-clamp-2 {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	
	.line-clamp-4 {
		display: -webkit-box;
		-webkit-line-clamp: 4;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	
	/* 季度网格容器滚动样式 */
	.seasons-grid-container {
		max-height: calc(120px * 5 + 1rem * 4); /* 5个横向行，每行120px高度，4个行间隔 */
		overflow-y: auto;
		padding-right: 0.5rem;
	}
	
	.seasons-grid-container::-webkit-scrollbar {
		width: 6px;
	}
	
	.seasons-grid-container::-webkit-scrollbar-track {
		background: #f1f1f1;
		border-radius: 3px;
	}
	
	.seasons-grid-container::-webkit-scrollbar-thumb {
		background: #c1c1c1;
		border-radius: 3px;
	}
	
	.seasons-grid-container::-webkit-scrollbar-thumb:hover {
		background: #a1a1a1;
	}
</style> 