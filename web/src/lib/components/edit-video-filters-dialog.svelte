<script lang="ts">
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { createEventDispatcher } from 'svelte';
	import { toast } from 'svelte-sonner';
	import type { UpdateVideoSourceFiltersRequest, VideoSourceFilterSettings } from '$lib/types';

	type Snapshot = {
		minDuration: string;
		maxDuration: string;
		minPageDuration: string;
		maxPageDuration: string;
		includeKeywords: string[];
		excludeKeywords: string[];
	};

	export let isOpen = false;
	export let sourceName = '';
	export let sourceType = '';
	export let currentFilters: VideoSourceFilterSettings = {
		min_duration_seconds: null,
		max_duration_seconds: null,
		min_page_duration_seconds: null,
		max_page_duration_seconds: null,
		include_keywords: [],
		exclude_keywords: []
	};

	const dispatch = createEventDispatcher<{
		confirm: UpdateVideoSourceFiltersRequest;
		cancel: void;
	}>();

	let minDurationInput = '';
	let maxDurationInput = '';
	let minPageDurationInput = '';
	let maxPageDurationInput = '';
	let includeKeywordsInput = '';
	let excludeKeywordsInput = '';
	let isProcessing = false;
	let initialSnapshot: Snapshot | null = null;
	let currentSnapshot: Snapshot;
	let hasChanges = false;
	let canConfirm = false;

	function formatNumber(value: number | null | undefined): string {
		return value !== null && value !== undefined ? `${value}` : '';
	}

	function keywordsToTextarea(keywords: string[] | null | undefined): string {
		return keywords && keywords.length > 0 ? keywords.join('\n') : '';
	}

	function resetState() {
		minDurationInput = formatNumber(currentFilters.min_duration_seconds);
		maxDurationInput = formatNumber(currentFilters.max_duration_seconds);
		minPageDurationInput = formatNumber(currentFilters.min_page_duration_seconds);
		maxPageDurationInput = formatNumber(currentFilters.max_page_duration_seconds);
		includeKeywordsInput = keywordsToTextarea(currentFilters.include_keywords);
		excludeKeywordsInput = keywordsToTextarea(currentFilters.exclude_keywords);
		isProcessing = false;
		initialSnapshot = snapshotState();
	}

	$: if (isOpen) {
		resetState();
	}

	function parseOptionalInteger(value: string, label: string): number | null | undefined {
		const trimmed = value.trim();
		if (!trimmed) {
			return null;
		}
		if (!/^\d+$/.test(trimmed)) {
			toast.error(`${label}必须是非负整数`);
			return undefined;
		}
		return Number.parseInt(trimmed, 10);
	}

	function parseKeywordInput(raw: string): string[] {
		return raw
			.split(/[\n,，,;；]+/)
			.map((kw) => kw.trim())
			.filter((kw) => kw.length > 0);
	}

	function snapshotState(): Snapshot {
		return {
			minDuration: minDurationInput.trim(),
			maxDuration: maxDurationInput.trim(),
			minPageDuration: minPageDurationInput.trim(),
			maxPageDuration: maxPageDurationInput.trim(),
			includeKeywords: parseKeywordInput(includeKeywordsInput),
			excludeKeywords: parseKeywordInput(excludeKeywordsInput)
		};
	}

	$: currentSnapshot = {
		minDuration: minDurationInput.trim(),
		maxDuration: maxDurationInput.trim(),
		minPageDuration: minPageDurationInput.trim(),
		maxPageDuration: maxPageDurationInput.trim(),
		includeKeywords: parseKeywordInput(includeKeywordsInput),
		excludeKeywords: parseKeywordInput(excludeKeywordsInput)
	};
	$: hasChanges = initialSnapshot
		? JSON.stringify(initialSnapshot) !== JSON.stringify(currentSnapshot)
		: true;
	$: canConfirm = hasChanges && !isProcessing;

	function getSourceTypeLabel(type: string): string {
		const typeMap: Record<string, string> = {
			collection: '合集',
			favorite: '收藏夹',
			submission: 'UP主投稿',
			watch_later: '稍后观看',
			bangumi: '番剧'
		};
		return typeMap[type] || type;
	}

	function handleCancel() {
		if (isProcessing) return;
		dispatch('cancel');
		isOpen = false;
	}

	function handleConfirm() {
		const minDurationValue = parseOptionalInteger(minDurationInput, '视频总时长下限');
		if (minDurationValue === undefined) {
			return;
		}
		const maxDurationValue = parseOptionalInteger(maxDurationInput, '视频总时长上限');
		if (maxDurationValue === undefined) {
			return;
		}
		if (
			minDurationValue !== null &&
			maxDurationValue !== null &&
			minDurationValue > maxDurationValue
		) {
			toast.error('视频总时长范围无效', { description: '下限不能大于上限' });
			return;
		}

		const minPageDurationValue = parseOptionalInteger(minPageDurationInput, '分P时长下限');
		if (minPageDurationValue === undefined) {
			return;
		}
		const maxPageDurationValue = parseOptionalInteger(maxPageDurationInput, '分P时长上限');
		if (maxPageDurationValue === undefined) {
			return;
		}
		if (
			minPageDurationValue !== null &&
			maxPageDurationValue !== null &&
			minPageDurationValue > maxPageDurationValue
		) {
			toast.error('分P时长范围无效', { description: '下限不能大于上限' });
			return;
		}

		const includeKeywords = parseKeywordInput(includeKeywordsInput);
		const excludeKeywords = parseKeywordInput(excludeKeywordsInput);

		isProcessing = true;
		try {
			dispatch('confirm', {
				min_duration_seconds: minDurationValue,
				max_duration_seconds: maxDurationValue,
				min_page_duration_seconds: minPageDurationValue,
				max_page_duration_seconds: maxPageDurationValue,
				include_keywords: includeKeywords.length > 0 ? includeKeywords : null,
				exclude_keywords: excludeKeywords.length > 0 ? excludeKeywords : null
			});
			isOpen = false;
		} catch (error) {
			console.error('更新过滤规则失败:', error);
			toast.error('更新失败', {
				description: error instanceof Error ? error.message : '请稍后重试'
			});
		} finally {
			isProcessing = false;
		}
	}
</script>

<AlertDialog.Root bind:open={isOpen}>
	<AlertDialog.Content class="max-w-2xl">
		<AlertDialog.Header>
			<AlertDialog.Title class="flex items-center gap-2 text-purple-600 dark:text-purple-400">
				<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M3 4h18M4 8h16l-1 12H5L4 8zm4 4h1m4 0h1M9 12v4m5-4v4"
					/>
				</svg>
				编辑视频过滤规则
			</AlertDialog.Title>
			<AlertDialog.Description class="space-y-4">
				<div
					class="rounded-lg border border-purple-200 bg-purple-50 p-3 text-xs text-purple-800 dark:border-purple-800 dark:bg-purple-950 dark:text-purple-200"
				>
					<p class="font-medium">通过过滤规则可以更精准地控制自动下载的视频范围。</p>
					<p class="mt-1">支持配置视频总时长、分P时长范围，以及标题关键词包含/排除规则。</p>
				</div>

				<div class="text-muted-foreground space-y-2 text-sm">
					<div>
						<span class="text-foreground font-medium">类型：</span>
						<span
							class="rounded bg-purple-100 px-2 py-0.5 text-xs text-purple-700 dark:bg-purple-900 dark:text-purple-100"
						>
							{getSourceTypeLabel(sourceType)}
						</span>
					</div>
					<div>
						<span class="text-foreground font-medium">名称：</span>
						<span class="text-foreground font-mono">"{sourceName}"</span>
					</div>
				</div>

				<div class="grid gap-4 md:grid-cols-2">
					<div class="space-y-2">
						<label class="text-foreground text-sm font-medium" for="min-duration"
							>视频总时长下限（秒）</label
						>
						<input
							id="min-duration"
							type="text"
							bind:value={minDurationInput}
							placeholder="留空表示不限"
							class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-purple-500 focus:ring-2 focus:ring-purple-500 focus:outline-none dark:border-gray-600 dark:bg-gray-700 dark:text-gray-200"
							disabled={isProcessing}
						/>
					</div>
					<div class="space-y-2">
						<label class="text-foreground text-sm font-medium" for="max-duration"
							>视频总时长上限（秒）</label
						>
						<input
							id="max-duration"
							type="text"
							bind:value={maxDurationInput}
							placeholder="留空表示不限"
							class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-purple-500 focus:ring-2 focus:ring-purple-500 focus:outline-none dark:border-gray-600 dark:bg-gray-700 dark:text-gray-200"
							disabled={isProcessing}
						/>
					</div>
					<div class="space-y-2">
						<label class="text-foreground text-sm font-medium" for="min-page-duration"
							>分P时长下限（秒）</label
						>
						<input
							id="min-page-duration"
							type="text"
							bind:value={minPageDurationInput}
							placeholder="留空表示不限"
							class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-purple-500 focus:ring-2 focus:ring-purple-500 focus:outline-none dark:border-gray-600 dark:bg-gray-700 dark:text-gray-200"
							disabled={isProcessing}
						/>
					</div>
					<div class="space-y-2">
						<label class="text-foreground text-sm font-medium" for="max-page-duration"
							>分P时长上限（秒）</label
						>
						<input
							id="max-page-duration"
							type="text"
							bind:value={maxPageDurationInput}
							placeholder="留空表示不限"
							class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-purple-500 focus:ring-2 focus:ring-purple-500 focus:outline-none dark:border-gray-600 dark:bg-gray-700 dark:text-gray-200"
							disabled={isProcessing}
						/>
					</div>
				</div>

				<div class="grid gap-4 md:grid-cols-2">
					<div class="space-y-2">
						<label class="text-foreground text-sm font-medium" for="include-keywords"
							>包含关键词</label
						>
						<textarea
							id="include-keywords"
							bind:value={includeKeywordsInput}
							placeholder="多个关键词用逗号、分号或换行分隔"
							rows="5"
							class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-purple-500 focus:ring-2 focus:ring-purple-500 focus:outline-none dark:border-gray-600 dark:bg-gray-700 dark:text-gray-200"
							disabled={isProcessing}
						></textarea>
						<p class="text-muted-foreground text-xs">
							所有关键词都会被转为小写后匹配标题，留空表示不过滤。
						</p>
					</div>
					<div class="space-y-2">
						<label class="text-foreground text-sm font-medium" for="exclude-keywords"
							>排除关键词</label
						>
						<textarea
							id="exclude-keywords"
							bind:value={excludeKeywordsInput}
							placeholder="多个关键词用逗号、分号或换行分隔"
							rows="5"
							class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-purple-500 focus:ring-2 focus:ring-purple-500 focus:outline-none dark:border-gray-600 dark:bg-gray-700 dark:text-gray-200"
							disabled={isProcessing}
						></textarea>
						<p class="text-muted-foreground text-xs">匹配到任意排除关键词的视频会被跳过下载。</p>
					</div>
				</div>

				<div
					class="rounded-lg border border-yellow-200 bg-yellow-50 p-3 text-xs text-yellow-800 dark:border-yellow-800 dark:bg-yellow-950 dark:text-yellow-200"
				>
					<p class="font-medium">提示</p>
					<ul class="mt-1 list-inside list-disc space-y-1">
						<li>所有过滤设置都会在下一次扫描该视频源时生效。</li>
						<li>留空或清空字段即可移除对应的过滤条件。</li>
					</ul>
				</div>
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer class="flex justify-end gap-3 pt-4">
			<button
				type="button"
				class="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 focus:outline-none disabled:cursor-not-allowed disabled:opacity-50 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-200 dark:hover:bg-gray-600 dark:focus:ring-offset-gray-800"
				on:click={handleCancel}
				disabled={isProcessing}
			>
				取消
			</button>
			<button
				type="button"
				class="rounded-md border border-transparent bg-purple-600 px-4 py-2 text-sm font-medium text-white hover:bg-purple-700 focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 focus:outline-none disabled:cursor-not-allowed disabled:opacity-50"
				on:click={handleConfirm}
				disabled={!canConfirm}
			>
				{#if isProcessing}
					<svg class="mr-2 inline h-4 w-4 animate-spin" fill="none" viewBox="0 0 24 24">
						<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"
						></circle>
						<path
							class="opacity-75"
							fill="currentColor"
							d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
						></path>
					</svg>
					保存中...
				{:else}
					保存过滤规则
				{/if}
			</button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
