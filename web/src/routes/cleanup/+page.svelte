<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { toast } from 'svelte-sonner';
	import type {
		LibraryUsage,
		LibraryVideo,
		UploaderUsage,
		UpdateVideoSourceFiltersRequest
	} from '$lib/types';
	import EditVideoFiltersDialog from '$lib/components/edit-video-filters-dialog.svelte';
	let data: LibraryUsage | null = null;
	let loading = false;
	let error = '';
	let search = '';
	let sort:
		'primary_bytes' | 'primary_video_count' | 'coauthored_bytes' | 'coauthored_video_count' =
		'primary_bytes';
	let ascending = false;
	let limit = 50;
	let selected: UploaderUsage | null = null;
	let videos: LibraryVideo[] = [];
	let videoLoading = false;
	let videoLimit = 50;
	let videoSearch = '';
	let showFilters = false;
	let saving = false;
	$: rows = (data?.uploaders ?? [])
		.filter((u) => `${u.name} ${u.uid}`.toLowerCase().includes(search.toLowerCase()))
		.sort((a, b) => (ascending ? 1 : -1) * (a[sort] - b[sort]) || a.name.localeCompare(b.name));
	$: visibleVideos = videos.filter((v) =>
		`${v.name} ${v.bvid}`.toLowerCase().includes(videoSearch.toLowerCase())
	);
	function bytes(value: number) {
		if (value < 1024) return `${value} B`;
		const i = Math.min(Math.floor(Math.log(value) / Math.log(1024)), 4);
		return `${(value / 1024 ** i).toFixed(1)} ${['B', 'KiB', 'MiB', 'GiB', 'TiB'][i]}`;
	}
	async function load() {
		loading = true;
		error = '';
		try {
			data = (await api.getLibraryUsage()).data;
		} catch (e) {
			error = e instanceof Error ? e.message : '统计失败';
		} finally {
			loading = false;
		}
	}
	async function inspect(uploader: UploaderUsage) {
		selected = uploader;
		videos = [];
		videoSearch = '';
		videoLimit = 50;
		videoLoading = true;
		try {
			const result = await api.getUploaderVideos(uploader.uid);
			if (selected?.uid === uploader.uid) videos = result.data;
		} catch (e) {
			toast.error(e instanceof Error ? e.message : '加载视频失败');
		} finally {
			if (selected?.uid === uploader.uid) videoLoading = false;
		}
	}
	async function save(filters: UpdateVideoSourceFiltersRequest) {
		if (!selected) return;
		const target = selected;
		saving = true;
		try {
			const result = await api.saveUploaderFilter(target.uid, filters);
			target.filters = result.data;
			if (data)
				data = {
					...data,
					uploaders: data.uploaders.map((u) => (u.uid === target.uid ? target : u))
				};
			if (selected?.uid === target.uid) selected = { ...target };
			toast.success('已保存此UP主的过滤规则');
		} catch (e) {
			toast.error(e instanceof Error ? e.message : '保存失败');
		} finally {
			saving = false;
		}
	}
	onMount(load);
</script>

<svelte:head><title>视频清理 - Bili Sync</title></svelte:head>
<div class="mx-auto max-w-7xl space-y-6 p-6">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<h1 class="text-2xl font-bold">视频清理</h1>
		<button
			class="rounded-md border px-4 py-2 disabled:opacity-50"
			onclick={load}
			disabled={loading}>{loading ? '正在统计…' : '刷新磁盘统计'}</button
		>
	</div>
	<p class="text-muted-foreground text-sm">
		按创作者查看视频文件体积和视频数。主投稿与联合创作分别统计；同一 BV
		的多个来源只计一个视频，独立文件副本分别计入体积。协作体积包含共享视频，不能与其他作者直接相加。不含封面、字幕和回收站文件。
	</p>
	{#if error}<p role="alert" class="text-destructive">{error}</p>{/if}
	{#if data}
		{#if data.unreadable_files > 0}<p class="text-amber-600">
				{data.unreadable_files} 个路径无法读取，体积统计不完整。
			</p>{/if}
		<div class="flex flex-wrap items-center gap-3">
			<input
				class="rounded-md border bg-transparent px-3 py-2"
				aria-label="搜索创作者"
				placeholder="搜索创作者"
				bind:value={search}
				oninput={() => (limit = 50)}
			/>
			<select
				class="bg-background rounded-md border px-3 py-2"
				aria-label="排序依据"
				bind:value={sort}
				onchange={() => (limit = 50)}
			>
				<option value="primary_bytes">主投稿体积</option><option value="primary_video_count"
					>主投稿视频数</option
				><option value="coauthored_bytes">联合创作体积</option><option
					value="coauthored_video_count">联合创作视频数</option
				>
			</select>
			<label class="flex items-center gap-2 text-sm"
				><input type="checkbox" bind:checked={ascending} />升序</label
			>
			<span class="text-muted-foreground text-sm">{rows.length} 位创作者</span>
		</div>
		<div class="overflow-x-auto rounded-lg border">
			<table class="w-full text-left text-sm">
				<thead class="bg-muted"
					><tr
						><th class="p-3">创作者</th><th class="p-3">主投稿体积</th><th class="p-3">视频数</th
						><th class="p-3">联合创作体积</th><th class="p-3">协作视频数</th><th class="p-3"
							>管理</th
						></tr
					></thead
				>
				<tbody
					>{#each rows.slice(0, limit) as row (row.uid)}
						<tr class="border-t"
							><td class="p-3">{row.name || '未知创作者'}</td><td class="p-3"
								>{bytes(row.primary_bytes)}</td
							><td class="p-3">{row.primary_video_count}</td><td class="p-3"
								>{bytes(row.coauthored_bytes)}</td
							><td class="p-3">{row.coauthored_video_count}</td><td class="p-3"
								><button class="text-primary underline" onclick={() => inspect(row)}
									>视频与过滤规则</button
								></td
							></tr
						>
					{:else}<tr
							><td colspan="6" class="text-muted-foreground p-8 text-center">暂无匹配的创作者</td
							></tr
						>{/each}</tbody
				>
			</table>
		</div>
		{#if rows.length > limit}<button
				class="rounded-md border px-4 py-2"
				onclick={() => (limit += 50)}>显示更多</button
			>{/if}
	{:else if loading}<p aria-live="polite">正在读取视频文件信息…</p>{/if}
	{#if selected}
		<section class="space-y-4 rounded-lg border p-5" aria-label="创作者管理">
			<div class="flex flex-wrap items-center justify-between gap-3">
				<h2 class="text-lg font-semibold">{selected.name}</h2>
				<button
					class="rounded-md border px-4 py-2"
					disabled={saving}
					onclick={() => (showFilters = true)}>编辑此UP主过滤规则</button
				>
			</div>
			<p class="text-muted-foreground text-sm">
				规则只作用于此UP主作为主投稿者的视频，不作用于仅参与的联合创作。与视频源规则共同生效；下次扫描时用于自动下载。已下载文件保留，可进入视频详情预览并删除。删除联合创作视频会删除整个文件。
			</p>

			<input
				class="rounded-md border bg-transparent px-3 py-2"
				aria-label="搜索此作者的视频"
				placeholder="搜索标题或 BV 号"
				bind:value={videoSearch}
			/>
			{#if videoLoading}<p>正在加载视频…</p>{:else}
				<ul class="divide-y">
					{#each visibleVideos.slice(0, videoLimit) as video (video.id)}<li class="py-3">
							<a class="text-primary hover:underline" href={`/video/${video.id}`}>{video.name}</a>
							<p class="text-muted-foreground mt-1 text-xs">
								{video.creators.map((c) => `${c.name}（${c.role}）`).join(' · ')}
							</p>
						</li>{:else}<li class="text-muted-foreground py-3">暂无匹配的视频</li>{/each}
				</ul>
				{#if visibleVideos.length > videoLimit}<button
						class="rounded-md border px-4 py-2"
						onclick={() => (videoLimit += 50)}>显示更多视频</button
					>{/if}
			{/if}
		</section>
		<EditVideoFiltersDialog
			bind:isOpen={showFilters}
			sourceName={selected.name}
			sourceType="创作者"
			currentFilters={{
				...selected.filters,
				min_duration_seconds: selected.filters.min_duration_seconds ?? null,
				max_duration_seconds: selected.filters.max_duration_seconds ?? null,
				min_page_duration_seconds: selected.filters.min_page_duration_seconds ?? null,
				max_page_duration_seconds: selected.filters.max_page_duration_seconds ?? null,
				include_keywords: selected.filters.include_keywords ?? [],
				exclude_keywords: selected.filters.exclude_keywords ?? []
			}}
			on:confirm={(event) => save(event.detail)}
		/>
	{/if}
</div>
