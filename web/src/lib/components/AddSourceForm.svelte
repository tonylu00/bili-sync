<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import { addVideoSource, getBangumiSeasons } from '$lib/api';
	import { toast } from 'svelte-sonner';
	import type { VideoCategory } from '$lib/types';

	export let onSuccess: () => void;

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
	}> = [];
	let selectedSeasons: string[] = [];
	
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
	
	// 切换季度选择
	function toggleSeasonSelection(seasonId: string) {
		const index = selectedSeasons.indexOf(seasonId);
		if (index === -1) {
			selectedSeasons = [...selectedSeasons, seasonId];
		} else {
			selectedSeasons = selectedSeasons.filter(id => id !== seasonId);
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
		
		{#if source_type === 'collection'}
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
		<div>
			<label class="block text-sm font-medium mb-1" for="source-id">
				{source_type === 'bangumi' ? 'season_id' : 
				  source_type === 'favorite' ? '收藏夹ID' : 
				  source_type === 'submission' ? 'UP主ID' : 
				  source_type === 'collection' ? '合集ID' : 'ID'}
			</label>
			<Input id="source-id" bind:value={source_id} placeholder="请输入ID" />
		</div>
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
				<span class="text-xs text-gray-500 ml-2">（不选择则下载当前输入的季度，不创建季度文件夹）</span>
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
								{season.season_title} (ID: {season.season_id})
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