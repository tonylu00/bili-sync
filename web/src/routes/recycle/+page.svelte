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
    selectedVideo = video;
    restoreDialogOpen = true;
  }

  async function handleRestore() {
    if (!selectedVideo) {
      return;
    }

    restoring = true;
    try {
      const response = await api.restoreVideo(selectedVideo.id);
      toast.success('恢复成功', {
        description: response.data.message
      });
      restoreDialogOpen = false;
      await loadDeletedVideos(currentPage);
    } catch (error) {
      toast.error('恢复失败', {
        description: (error as ApiError).message
      });
    } finally {
      restoring = false;
    }
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
</script>

<div class="space-y-6">
  <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
    <div>
      <h1 class="text-2xl font-semibold">视频回收站</h1>
      <p class="text-muted-foreground text-sm">
        查看并恢复被标记为已删除的视频，恢复后系统将在下次扫描时重新下载文件。
      </p>
    </div>
    {#if recycleData}
      <div class="text-muted-foreground text-sm">
        共 {recycleData.total_count} 个已删除视频
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-16">
      <div class="text-muted-foreground">正在加载回收站数据...</div>
    </div>
  {:else if recycleData && recycleData.videos.length > 0}
    <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
      {#each recycleData.videos as video (video.id)}
        <div class="flex flex-col gap-3">
          <VideoCard video={video} showActions={false} showProgress={false} mode="detail" />
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
        确定要恢复视频「{selectedVideoName}」吗？系统将重新创建下载任务。
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel>取消</AlertDialog.Cancel>
      <Button on:click={handleRestore} disabled={restoring}>
        {restoring ? '恢复中…' : '恢复'}
      </Button>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
