<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import api from '$lib/api';
	import type { UploaderNameResponse } from '$lib/types';
	import { toast } from 'svelte-sonner';

	export let isOpen = true;
	export let videoId: number;
	const dispatch = createEventDispatcher<{ renamed: { videoId: number; name: string } }>();
	let uploader: UploaderNameResponse | null = null;
	let name = '';
	let loading = true;
	let saving = false;
	let errorMessage = '';
	$: validName =
		name.trim().length > 0 &&
		[...name.trim()].length <= 100 &&
		[...name].every((character) => {
			const code = character.codePointAt(0)!;
			return code >= 32 && (code < 127 || code > 159);
		});

	onMount(async () => {
		try {
			const result = await api.getUploaderName(videoId);
			uploader = result.data;
			name = uploader.name;
		} catch {
			errorMessage = '无法读取UP主信息。视频需要有已保存的UP主UID，请稍后重试。';
		} finally {
			loading = false;
		}
	});

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (!uploader || !validName || saving) return;
		saving = true;
		errorMessage = '';
		try {
			const result = await api.renameUploader(videoId, name.trim(), uploader.upper_id);
			dispatch('renamed', { videoId, name: result.data.name });
			toast.success(`已更新 ${result.data.video_count} 个视频的UP主名称`);
			isOpen = false;
		} catch {
			errorMessage = '保存失败，请重新打开编辑窗口后重试。';
		} finally {
			saving = false;
		}
	}
</script>

<AlertDialog.Root bind:open={isOpen}>
	<AlertDialog.Content class="max-h-[90vh] overflow-y-auto">
		<AlertDialog.Header>
			<AlertDialog.Title>修改UP主名称</AlertDialog.Title>
			<AlertDialog.Description>
				可根据视频水印填写原名称，或选择已记录的名称。修改会应用到该UP主的所有视频及投稿源，后续扫描也会保留。
			</AlertDialog.Description>
		</AlertDialog.Header>
		<form onsubmit={save} class="space-y-4">
			{#if loading}
				<p role="status" class="text-muted-foreground text-sm">正在读取UP主信息…</p>
			{:else if uploader}
				<p class="text-muted-foreground text-sm">
					UID：{uploader.upper_id} · 共 {uploader.video_count} 个视频
				</p>
				<div class="space-y-2">
					<label for="uploader-name" class="text-sm font-medium">UP主名称</label>
					<Input
						id="uploader-name"
						bind:value={name}
						disabled={saving}
						required
						placeholder="填写视频水印中的名称"
					/>
					<p class="text-muted-foreground text-xs">1–100个字符。已有文件名和视频水印不会修改。</p>
				</div>
				{#if uploader.names.length > 0}
					<div class="space-y-2">
						<p class="text-sm font-medium">已记录的名称</p>
						<div class="flex max-h-40 flex-wrap gap-2 overflow-y-auto">
							{#each uploader.names as previousName (previousName)}
								<Button
									type="button"
									size="sm"
									variant="outline"
									disabled={saving}
									onclick={() => (name = previousName)}
									class="h-auto max-w-full break-all whitespace-normal">{previousName}</Button
								>
							{/each}
						</div>
					</div>
				{/if}
			{/if}
			{#if errorMessage}<p role="alert" class="text-destructive text-sm">{errorMessage}</p>{/if}
			<AlertDialog.Footer>
				<Button type="button" variant="outline" disabled={saving} onclick={() => (isOpen = false)}
					>取消</Button
				>
				<Button type="submit" disabled={loading || saving || !uploader || !validName}
					>{saving ? '保存中…' : '应用到该UP主的所有视频'}</Button
				>
			</AlertDialog.Footer>
		</form>
	</AlertDialog.Content>
</AlertDialog.Root>
