<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { setBreadcrumb } from '$lib/stores/breadcrumb';
  import api from '$lib/api';
  import VideoCard from '$lib/components/video-card.svelte';
  import Pagination from '$lib/components/pagination.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import Undo2Icon from '@lucide/svelte/icons/undo-2';
  import CheckIcon from '@lucide/svelte/icons/check';
  import XIcon from '@lucide/svelte/icons/x';
  import { toast } from 'svelte-sonner';
  import type { VideosResponse, VideoInfo, ApiError } from '$lib/types';

  const pageSize = 20;

  let recycleData: VideosResponse | null = null;
  let loading = false;
  let currentPage = 0;
  let totalPages = 1;
  let restoreDialogOpen = false;
  let restoring = false;
  let selectedVideo: VideoInfo | null = null;
  let dialogVideos: VideoInfo[] = [];
  let restoreMode: 'single' | 'batch' = 'single';
  let selectedVideoIds: number[] = [];
  let selectionMode = false;
  let selectedVideos: Set<number> = new Set();
  let requestToken = 0;
  let unsubscribe = () => {};

  function getPageFromSearch(searchParams: URLSearchParams): number {
    const pageParam = Number.parseInt(searchParams.get('page') ?? '0', 10);
    if (Number.isNaN(pageParam) || pageParam < 0) {
      return 0;
    }
    return pageParam;
  }

  async function loadDeletedVideos(pageNum: number) {
    loading = true;
    const token = ++requestToken;
    try {
      const result = await api.getDeletedVideos({
        page: pageNum,
        page_size: pageSize,
        sort_by: 'id',
        sort_order: 'desc'
      });

      if (token !== requestToken) {
        return;
      }

      recycleData = result.data;
      if (selectionMode) {
        const availableIds = new Set(result.data.videos.map((video) => video.id));
        const filtered = [...selectedVideos].filter((id) => availableIds.has(id));
        selectedVideos = new Set(filtered);
      }
      totalPages = Math.max(1, Math.ceil(result.data.total_count / pageSize));
      loading = false;

      if (
        recycleData.videos.length === 0 &&
        result.data.total_count > 0 &&
        pageNum >= totalPages &&
        pageNum > 0
      ) {
        goto(`/recycle?page=${totalPages - 1}`);
      }
    } catch (error) {
      if (token !== requestToken) {
        return;
      }

      loading = false;
      recycleData = null;
      toast.error('加载回收站失败', {
        description: (error as ApiError).message
      });
    }
  }

  function handlePageChange(pageNum: number) {
    goto(`/recycle?page=${pageNum}`);
  }

  function openRestoreDialog(video: VideoInfo) {
    restoreMode = 'single';
    selectedVideo = video;
    dialogVideos = [video];
    selectedVideoIds = [video.id];
    restoreDialogOpen = true;
  }

  async function handleRestore() {
    restoring = true;
    try {
      const ids = [...selectedVideoIds];
      if (ids.length === 0) {
        toast.error('没有待恢复的视频');
        restoring = false;
        return;
      }
      let successCount = 0;
      const failed: { id: number; name: string; error: string }[] = [];

      for (const id of ids) {
        const videoInfo = recycleData?.videos.find((item) => item.id === id);
        const name = videoInfo?.name ?? `ID ${id}`;
        try {
          const response = await api.restoreVideo(id);
          successCount += 1;
          if (restoreMode === 'single') {
            toast.success('恢复成功', {
              description: response.data.message
            });
          }
        } catch (error) {
          failed.push({
            id,
            name,
            error: (error as ApiError).message
          });
        }
      }

      if (restoreMode === 'batch') {
        if (successCount > 0 && failed.length === 0) {
          toast.success('批量恢复成功', {
            description: `成功恢复 ${successCount} 个视频`
          });
        } else if (successCount > 0 && failed.length > 0) {
          const failedNames = failed.map((item) => item.name).join('、');
          toast.error('部分恢复失败', {
            description: `成功 ${successCount} 个，失败 ${failed.length} 个：${failedNames}`
          });
        } else if (failed.length > 0) {
          const failedNames = failed.map((item) => item.name).join('、');
          toast.error('恢复失败', {
            description: `无法恢复：${failedNames}`
          });
        }
      } else if (restoreMode === 'single' && failed.length > 0) {
        toast.error('恢复失败', {
          description: failed[0].error
        });
      }

      restoreDialogOpen = false;
      selectedVideoIds = [];
      dialogVideos = [];
      selectedVideo = null;
      if (selectionMode) {
        selectedVideos = new Set([...selectedVideos].filter((id) => !ids.includes(id)));
      }
      await loadDeletedVideos(currentPage);
    } catch (error) {
      toast.error('恢复失败', {
        description: (error as ApiError).message
      });
    } finally {
      restoring = false;
    }
  }

  function toggleSelectionMode() {
    selectionMode = !selectionMode;
    if (!selectionMode) {
      selectedVideos = new Set();
    }
  }

  function handleSelectionChange(videoId: number, isSelected: boolean) {
    const next = new Set(selectedVideos);
    if (isSelected) {
      next.add(videoId);
    } else {
      next.delete(videoId);
    }
    selectedVideos = next;
  }

  function selectAllCurrentPage() {
    if (!recycleData) {
      return;
    }
    selectedVideos = new Set(recycleData.videos.map((video) => video.id));
  }

  function clearSelection() {
    selectedVideos = new Set();
  }

  function openBatchRestoreDialog() {
    if (selectedVideos.size === 0) {
      toast.error('请先选择需要恢复的视频');
      return;
    }

    if (!recycleData) {
      toast.error('暂无可恢复的视频');
      return;
    }

    restoreMode = 'batch';
    selectedVideo = null;
    const selectedIds = Array.from(selectedVideos);
    selectedVideoIds = selectedIds;
    dialogVideos = recycleData.videos.filter((video) => selectedVideos.has(video.id));
    restoreDialogOpen = true;
  }

  onMount(() => {
    setBreadcrumb([{ label: '视频回收站' }]);

    unsubscribe = page.subscribe(($page) => {
      const newPage = getPageFromSearch($page.url.searchParams);
      const shouldReload = newPage !== currentPage || recycleData === null;
      currentPage = newPage;
      if (shouldReload) {
        loadDeletedVideos(currentPage);
      }
    });
  });

  onDestroy(() => {
    unsubscribe();
  });

  $: selectedVideoName = selectedVideo ? selectedVideo.name : '';
  $: selectedCount = selectedVideos.size;
  const PREVIEW_LIMIT = 5;
</script>

<div class="space-y-6">
  <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
    <div>
      <h1 class="text-2xl font-semibold">视频回收站</h1>
      <p class="text-muted-foreground text-sm">
        查看并恢复被标记为已删除的视频，恢复后系统将在下次扫描时重新下载文件。
      </p>
    </div>
    <div class="flex flex-col items-start gap-2 sm:items-end">
      {#if recycleData}
        <div class="text-muted-foreground text-sm">
          共 {recycleData.total_count} 个已删除视频
        </div>
      {/if}
      <div class="flex flex-wrap gap-2">
        {#if selectionMode}
          <Button size="sm" variant="outline" on:click={selectAllCurrentPage} disabled={!recycleData || recycleData.videos.length === 0}>
            <CheckIcon class="mr-1 h-4 w-4" />
            全选当前页
          </Button>
          <Button size="sm" variant="outline" on:click={clearSelection} disabled={selectedCount === 0}>
            <XIcon class="mr-1 h-4 w-4" />
            清除选择
          </Button>
          <Button size="sm" on:click={openBatchRestoreDialog} disabled={selectedCount === 0}>
            <Undo2Icon class="mr-1 h-4 w-4" />
            批量恢复 ({selectedCount})
          </Button>
          <Button size="sm" variant="ghost" on:click={toggleSelectionMode}>
            退出批量
          </Button>
        {:else}
          <Button
            size="sm"
            variant="outline"
            on:click={toggleSelectionMode}
            disabled={!recycleData || recycleData.videos.length === 0}
          >
            批量勾选
          </Button>
        {/if}
      </div>
    </div>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-16">
      <div class="text-muted-foreground">正在加载回收站数据...</div>
    </div>
  {:else if recycleData && recycleData.videos.length > 0}
    <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
      {#each recycleData.videos as video (video.id)}
        <div class="flex flex-col gap-3">
          <VideoCard
            video={video}
            showActions={false}
            showProgress={false}
            mode="detail"
            {selectionMode}
            selected={selectedVideos.has(video.id)}
            onSelectionChange={(id, isSelected) => handleSelectionChange(id, isSelected)}
          />
          <div class="flex justify-end">
            <Button size="sm" variant="outline" on:click={() => openRestoreDialog(video)}>
              <Undo2Icon class="mr-1 h-4 w-4" />
              恢复视频
            </Button>
          </div>
        </div>
      {/each}
    </div>

    {#if totalPages > 1}
      <Pagination {currentPage} {totalPages} onPageChange={handlePageChange} />
    {/if}
  {:else}
    <div class="flex items-center justify-center py-16">
      <div class="space-y-2 text-center">
        <div class="text-muted-foreground">回收站暂无视频</div>
        <p class="text-muted-foreground text-sm">删除的视频会出现在这里，您可以随时恢复它们。</p>
      </div>
    </div>
  {/if}
</div>

<AlertDialog.Root bind:open={restoreDialogOpen}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>确认恢复</AlertDialog.Title>
      <AlertDialog.Description>
        {#if restoreMode === 'single'}
          确定要恢复视频「{selectedVideoName}」吗？系统将重新创建下载任务。
        {:else}
          已选择 {selectedVideoIds.length} 个视频，确定要批量恢复吗？
          {#if dialogVideos.length > 0}
            <div class="text-muted-foreground mt-2 space-y-1 text-xs">
              {#each dialogVideos.slice(0, PREVIEW_LIMIT) as video}
                <div>• {video.name}</div>
              {/each}
              {#if selectedVideoIds.length > PREVIEW_LIMIT}
                <div>… 等 {selectedVideoIds.length} 个视频</div>
              {/if}
            </div>
          {/if}
        {/if}
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel>取消</AlertDialog.Cancel>
      <Button on:click={handleRestore} disabled={restoring}>
        {restoring ? '恢复中…' : restoreMode === 'single' ? '恢复' : '批量恢复'}
      </Button>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
