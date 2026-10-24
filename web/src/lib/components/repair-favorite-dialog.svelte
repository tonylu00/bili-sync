<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import api from '$lib/api';
	import type { ApiError } from '$lib/types';
	import { toast } from 'svelte-sonner';

	export let isOpen = false;
	export let sourceId = 0;
	export let sourceName = '';
	export let currentId = '';
	export let currentOwner = '';
	const dispatch = createEventDispatcher<{ repaired: void }>();
	let favorite = '';
	let creatorUid = '';
	type Candidate = { title: string; creator_uid: string; url: string };
	let candidate: Candidate | null = null;
	let candidates: Candidate[] = [];
	let saving = false;
	let errorMessage = '';

	$: if (isOpen) {
		favorite = '';
		creatorUid = currentOwner;
		candidate = null;
		candidates = [];
		errorMessage = '';
	}

	function selectCandidate(value: Candidate) {
		candidate = value;
		favorite = value.url;
		creatorUid = value.creator_uid;
	}

	async function repair(automatic = false, allowProbe = false) {
		if (saving || (!automatic && !favorite.trim())) return;
		saving = true;
		errorMessage = '';
		try {
			const result = await api.repairFavoriteSource(
				sourceId,
				currentId,
				automatic ? '' : favorite.trim(),
				creatorUid.trim(),
				allowProbe
			);
			if (result.data.candidates?.length) {
				candidates = result.data.candidates;
				candidate = null;
				favorite = '';
				creatorUid = currentOwner;
				if (candidates.length === 1) selectCandidate(candidates[0]);
				errorMessage = result.data.message;
				return;
			}
			if (!result.data.success) {
				errorMessage = result.data.message;
				return;
			}
			toast.success(result.data.message);
			isOpen = false;
			dispatch('repaired');
		} catch (error) {
			errorMessage = (error as ApiError)?.message || '修复失败，请重试';
		} finally {
			saving = false;
		}
	}
</script>

<AlertDialog.Root bind:open={isOpen}>
	<AlertDialog.Content class="max-h-[90vh] overflow-y-auto">
		<AlertDialog.Header>
			<AlertDialog.Title>修复收藏夹ID</AlertDialog.Title>
			<AlertDialog.Description>
				为「{sourceName}」（当前ID：{currentId}）自动查找完整ID，或填写原收藏夹URL。
				修复后保留下载记录、保存路径和过滤设置，下次扫描使用正确的收藏夹。
			</AlertDialog.Description>
		</AlertDialog.Header>
		<div class="space-y-2">
			<p class="text-muted-foreground text-sm">
				自动修复使用已知创建者UID，或匹配当前账号创建和关注的收藏夹。多个匹配时需指定创建者。
			</p>
			<label for="repair-favorite-owner" class="text-sm font-medium">创建者UID（可选）</label>
			<Input
				id="repair-favorite-owner"
				bind:value={creatorUid}
				placeholder="已知原收藏夹创建者时填写"
				disabled={saving}
			/>
			<label for="repair-favorite-url" class="text-sm font-medium">收藏夹URL或完整ID</label>
			<Input
				id="repair-favorite-url"
				bind:value={favorite}
				placeholder="https://space.bilibili.com/…/favlist?fid=…"
				disabled={saving}
			/>
			<p class="text-muted-foreground text-sm">打开B站原收藏夹，复制浏览器地址栏中的完整URL。</p>
			<details>
				<summary class="text-sm">旧收藏夹候选查找（可选）</summary>
				<p class="text-muted-foreground text-sm">
					仅用于旧收藏夹：尝试00–99尾号，最多查询100次。短ID可能被不同创建者复用，找到候选后需核对创建者及URL再确认，不会直接修改源。
				</p>
				<Button variant="outline" disabled={saving} onclick={() => repair(true, true)}
					>查找旧收藏夹候选</Button
				>
			</details>
			{#each candidates as item (item.url)}
				<div class="rounded border p-2 text-sm">
					<p>{item.title} · 创建者UID：{item.creator_uid}</p>
					<a class="underline" href={item.url} target="_blank" rel="noreferrer">打开收藏夹核对</a>
					<Button variant="outline" disabled={saving} onclick={() => selectCandidate(item)}>
						{candidate?.url === item.url ? '已选择' : '选择此收藏夹'}
					</Button>
				</div>
			{/each}
			{#if errorMessage}
				<p role="alert" class="text-destructive text-sm">{errorMessage}</p>
			{/if}
		</div>
		<AlertDialog.Footer>
			<Button variant="outline" disabled={saving} onclick={() => (isOpen = false)}>取消</Button>
			<Button variant="outline" disabled={saving || !favorite.trim()} onclick={() => repair()}>
				{saving ? '验证并修复中…' : candidate ? '确认候选并修复' : '验证并修复'}
			</Button>
			<Button disabled={saving || candidates.length > 0} onclick={() => repair(true)}>
				{saving ? '查找并修复中…' : '自动查找并修复'}
			</Button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
