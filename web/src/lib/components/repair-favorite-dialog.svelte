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
	const dispatch = createEventDispatcher<{ repaired: void }>();
	let favorite = '';
	let saving = false;
	let errorMessage = '';

	$: if (isOpen) {
		favorite = '';
		errorMessage = '';
	}

	async function repair() {
		if (saving || !favorite.trim()) return;
		saving = true;
		errorMessage = '';
		try {
			const result = await api.repairFavoriteSource(sourceId, currentId, favorite.trim());
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
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>修复收藏夹ID</AlertDialog.Title>
			<AlertDialog.Description>
				为「{sourceName}」（当前ID：{currentId}）填写正确的收藏夹URL或完整ID。
				修复后保留下载记录、保存路径和过滤设置，下次扫描使用正确的收藏夹。
			</AlertDialog.Description>
		</AlertDialog.Header>
		<div class="space-y-2">
			<label for="repair-favorite-url" class="text-sm font-medium">收藏夹URL或完整ID</label>
			<Input
				id="repair-favorite-url"
				bind:value={favorite}
				placeholder="https://space.bilibili.com/…/favlist?fid=…"
				disabled={saving}
			/>
			<p class="text-muted-foreground text-sm">打开B站原收藏夹，复制浏览器地址栏中的完整URL。</p>
			{#if errorMessage}
				<p role="alert" class="text-destructive text-sm">{errorMessage}</p>
			{/if}
		</div>
		<AlertDialog.Footer>
			<Button variant="outline" disabled={saving} onclick={() => (isOpen = false)}>取消</Button>
			<Button disabled={saving || !favorite.trim()} onclick={repair}>
				{saving ? '验证并修复中…' : '验证并修复'}
			</Button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
