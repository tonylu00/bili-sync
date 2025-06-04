<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { RefreshCw, ListTodo, Settings, Plus, Trash2, Clock, CheckCircle, AlertCircle } from '@lucide/svelte';
	import api from '$lib/api';
	import { toast } from 'svelte-sonner';
	import type { QueueStatusResponse, QueueTaskInfo } from '$lib/types';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';

	let queueStatus: QueueStatusResponse | null = null;
	let loading = true;
	let error: string | null = null;
	let refreshInterval: number | null = null;

	// 设置面包屑
	setBreadcrumb([
		{ label: '首页', href: '/' },
		{ label: '任务队列', href: '/queue' }
	]);

	// 获取队列状态
	async function fetchQueueStatus() {
		try {
			const response = await api.getQueueStatus();
			queueStatus = response.data;
			error = null;
		} catch (err: any) {
			console.error('获取队列状态失败:', err);
			error = err.message || '获取队列状态失败';
			toast.error('获取队列状态失败', {
				description: err.message
			});
		} finally {
			loading = false;
		}
	}

	// 手动刷新
	async function handleRefresh() {
		loading = true;
		await fetchQueueStatus();
	}

	// 格式化时间
	function formatTime(timeString: string): string {
		try {
			const date = new Date(timeString);
			return date.toLocaleTimeString('zh-CN', {
				hour: '2-digit',
				minute: '2-digit',
				second: '2-digit'
			});
		} catch {
			return '无效时间';
		}
	}

	// 获取任务类型的显示名称
	function getTaskTypeName(taskType: string): string {
		const typeMap: Record<string, string> = {
			'delete_video_source': '删除视频源',
			'add_video_source': '添加视频源',
			'update_config': '更新配置',
			'reload_config': '重载配置'
		};
		return typeMap[taskType] || taskType;
	}

	// 获取任务类型的图标
	function getTaskTypeIcon(taskType: string) {
		const iconMap: Record<string, any> = {
			'delete_video_source': Trash2,
			'add_video_source': Plus,
			'update_config': Settings,
			'reload_config': RefreshCw
		};
		return iconMap[taskType] || ListTodo;
	}

	// 获取队列状态颜色
	function getQueueStatusVariant(isProcessing: boolean, hasItems: boolean): 'default' | 'secondary' | 'destructive' | 'outline' {
		if (isProcessing) return 'destructive';
		if (hasItems) return 'secondary';
		return 'outline';
	}

	// 获取队列状态文本
	function getQueueStatusText(isProcessing: boolean, hasItems: boolean): string {
		if (isProcessing) return '处理中';
		if (hasItems) return '等待中';
		return '空闲';
	}

	onMount(() => {
		fetchQueueStatus();
		// 每5秒自动刷新一次
		refreshInterval = setInterval(fetchQueueStatus, 5000);
	});

	onDestroy(() => {
		if (refreshInterval) {
			clearInterval(refreshInterval);
		}
	});
</script>

<div class="container max-w-6xl">
	<div class="flex items-center justify-between mb-6">
		<div>
			<h1 class="text-3xl font-bold">任务队列</h1>
			<p class="text-muted-foreground mt-2">查看和管理系统任务队列状态</p>
		</div>
		<Button variant="outline" size="sm" onclick={handleRefresh} disabled={loading}>
			<RefreshCw class="h-4 w-4 mr-2 {loading ? 'animate-spin' : ''}" />
			刷新
		</Button>
	</div>

	{#if error}
		<Card class="border-destructive">
			<CardContent class="pt-6">
				<div class="flex items-center gap-3 text-destructive">
					<AlertCircle class="h-5 w-5" />
					<span>{error}</span>
				</div>
			</CardContent>
		</Card>
	{:else if loading && !queueStatus}
		<div class="flex justify-center py-8">
			<div class="flex items-center gap-3">
				<RefreshCw class="h-5 w-5 animate-spin" />
				<span>加载队列状态中...</span>
			</div>
		</div>
	{:else if queueStatus}
		<!-- 系统状态总览 -->
		<div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4 mb-6">
			<Card>
				<CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle class="text-sm font-medium">扫描状态</CardTitle>
					{#if queueStatus.is_scanning}
						<Clock class="h-4 w-4 text-orange-500" />
					{:else}
						<CheckCircle class="h-4 w-4 text-green-500" />
					{/if}
				</CardHeader>
				<CardContent>
					<div class="text-2xl font-bold">
						{queueStatus.is_scanning ? '扫描中' : '空闲'}
					</div>
					<p class="text-xs text-muted-foreground">
						{queueStatus.is_scanning ? '正在扫描视频源' : '等待下次扫描'}
					</p>
				</CardContent>
			</Card>

			<Card>
				<CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle class="text-sm font-medium">删除队列</CardTitle>
					<Trash2 class="h-4 w-4 text-muted-foreground" />
				</CardHeader>
				<CardContent>
					<div class="text-2xl font-bold">{queueStatus.delete_queue.length}</div>
					<div class="flex items-center gap-2">
						<Badge variant={getQueueStatusVariant(queueStatus.delete_queue.is_processing, queueStatus.delete_queue.length > 0)}>
							{getQueueStatusText(queueStatus.delete_queue.is_processing, queueStatus.delete_queue.length > 0)}
						</Badge>
					</div>
				</CardContent>
			</Card>

			<Card>
				<CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle class="text-sm font-medium">添加队列</CardTitle>
					<Plus class="h-4 w-4 text-muted-foreground" />
				</CardHeader>
				<CardContent>
					<div class="text-2xl font-bold">{queueStatus.add_queue.length}</div>
					<div class="flex items-center gap-2">
						<Badge variant={getQueueStatusVariant(queueStatus.add_queue.is_processing, queueStatus.add_queue.length > 0)}>
							{getQueueStatusText(queueStatus.add_queue.is_processing, queueStatus.add_queue.length > 0)}
						</Badge>
					</div>
				</CardContent>
			</Card>

			<Card>
				<CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
					<CardTitle class="text-sm font-medium">配置队列</CardTitle>
					<Settings class="h-4 w-4 text-muted-foreground" />
				</CardHeader>
				<CardContent>
					<div class="text-2xl font-bold">{queueStatus.config_queue.update_length + queueStatus.config_queue.reload_length}</div>
					<div class="flex items-center gap-2">
						<Badge variant={getQueueStatusVariant(queueStatus.config_queue.is_processing, queueStatus.config_queue.update_length + queueStatus.config_queue.reload_length > 0)}>
							{getQueueStatusText(queueStatus.config_queue.is_processing, queueStatus.config_queue.update_length + queueStatus.config_queue.reload_length > 0)}
						</Badge>
					</div>
				</CardContent>
			</Card>
		</div>

		<!-- 队列详情 -->
		<div class="grid gap-6 lg:grid-cols-2">
			<!-- 删除队列 -->
			<Card>
				<CardHeader>
					<CardTitle class="flex items-center gap-2">
						<Trash2 class="h-5 w-5" />
						删除队列
						{#if queueStatus.delete_queue.is_processing}
							<Badge variant="destructive">处理中</Badge>
						{/if}
					</CardTitle>
					<CardDescription>
						等待处理的视频源删除任务
					</CardDescription>
				</CardHeader>
				<CardContent>
					{#if queueStatus.delete_queue.tasks.length === 0}
						<div class="flex items-center justify-center py-8 text-muted-foreground">
							<div class="text-center">
								<CheckCircle class="h-12 w-12 mx-auto mb-3 opacity-50" />
								<p>队列为空</p>
							</div>
						</div>
					{:else}
						<div class="space-y-3">
							{#each queueStatus.delete_queue.tasks as task (task.task_id)}
								<div class="flex items-center justify-between p-3 bg-muted/50 rounded-lg">
									<div class="flex items-center gap-3">
										<svelte:component this={getTaskTypeIcon(task.task_type)} class="h-4 w-4 text-muted-foreground" />
										<div>
											<p class="text-sm font-medium">{getTaskTypeName(task.task_type)}</p>
											<p class="text-xs text-muted-foreground">ID: {task.task_id}</p>
										</div>
									</div>
									<div class="text-right">
										<p class="text-xs text-muted-foreground">{formatTime(task.created_at)}</p>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</CardContent>
			</Card>

			<!-- 添加队列 -->
			<Card>
				<CardHeader>
					<CardTitle class="flex items-center gap-2">
						<Plus class="h-5 w-5" />
						添加队列
						{#if queueStatus.add_queue.is_processing}
							<Badge variant="destructive">处理中</Badge>
						{/if}
					</CardTitle>
					<CardDescription>
						等待处理的视频源添加任务
					</CardDescription>
				</CardHeader>
				<CardContent>
					{#if queueStatus.add_queue.tasks.length === 0}
						<div class="flex items-center justify-center py-8 text-muted-foreground">
							<div class="text-center">
								<CheckCircle class="h-12 w-12 mx-auto mb-3 opacity-50" />
								<p>队列为空</p>
							</div>
						</div>
					{:else}
						<div class="space-y-3">
							{#each queueStatus.add_queue.tasks as task (task.task_id)}
								<div class="flex items-center justify-between p-3 bg-muted/50 rounded-lg">
									<div class="flex items-center gap-3">
										<svelte:component this={getTaskTypeIcon(task.task_type)} class="h-4 w-4 text-muted-foreground" />
										<div>
											<p class="text-sm font-medium">{getTaskTypeName(task.task_type)}</p>
											<p class="text-xs text-muted-foreground">ID: {task.task_id}</p>
										</div>
									</div>
									<div class="text-right">
										<p class="text-xs text-muted-foreground">{formatTime(task.created_at)}</p>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</CardContent>
			</Card>

			<!-- 配置队列 -->
			<Card class="lg:col-span-2">
				<CardHeader>
					<CardTitle class="flex items-center gap-2">
						<Settings class="h-5 w-5" />
						配置队列
						{#if queueStatus.config_queue.is_processing}
							<Badge variant="destructive">处理中</Badge>
						{/if}
					</CardTitle>
					<CardDescription>
						等待处理的配置更新和重载任务
					</CardDescription>
				</CardHeader>
				<CardContent>
					{#if queueStatus.config_queue.update_tasks.length === 0 && queueStatus.config_queue.reload_tasks.length === 0}
						<div class="flex items-center justify-center py-8 text-muted-foreground">
							<div class="text-center">
								<CheckCircle class="h-12 w-12 mx-auto mb-3 opacity-50" />
								<p>队列为空</p>
							</div>
						</div>
					{:else}
						<div class="grid gap-4 md:grid-cols-2">
							<!-- 更新配置任务 -->
							<div>
								<h4 class="text-sm font-medium mb-3">更新配置任务 ({queueStatus.config_queue.update_length})</h4>
								{#if queueStatus.config_queue.update_tasks.length === 0}
									<p class="text-sm text-muted-foreground">暂无任务</p>
								{:else}
									<div class="space-y-3">
										{#each queueStatus.config_queue.update_tasks as task (task.task_id)}
											<div class="flex items-center justify-between p-3 bg-muted/50 rounded-lg">
												<div class="flex items-center gap-3">
													<svelte:component this={getTaskTypeIcon(task.task_type)} class="h-4 w-4 text-muted-foreground" />
													<div>
														<p class="text-sm font-medium">{getTaskTypeName(task.task_type)}</p>
														<p class="text-xs text-muted-foreground">ID: {task.task_id}</p>
													</div>
												</div>
												<div class="text-right">
													<p class="text-xs text-muted-foreground">{formatTime(task.created_at)}</p>
												</div>
											</div>
										{/each}
									</div>
								{/if}
							</div>

							<!-- 重载配置任务 -->
							<div>
								<h4 class="text-sm font-medium mb-3">重载配置任务 ({queueStatus.config_queue.reload_length})</h4>
								{#if queueStatus.config_queue.reload_tasks.length === 0}
									<p class="text-sm text-muted-foreground">暂无任务</p>
								{:else}
									<div class="space-y-3">
										{#each queueStatus.config_queue.reload_tasks as task (task.task_id)}
											<div class="flex items-center justify-between p-3 bg-muted/50 rounded-lg">
												<div class="flex items-center gap-3">
													<svelte:component this={getTaskTypeIcon(task.task_type)} class="h-4 w-4 text-muted-foreground" />
													<div>
														<p class="text-sm font-medium">{getTaskTypeName(task.task_type)}</p>
														<p class="text-xs text-muted-foreground">ID: {task.task_id}</p>
													</div>
												</div>
												<div class="text-right">
													<p class="text-xs text-muted-foreground">{formatTime(task.created_at)}</p>
												</div>
											</div>
										{/each}
									</div>
								{/if}
							</div>
						</div>
					{/if}
				</CardContent>
			</Card>
		</div>

		<!-- 说明信息 -->
		<Card class="mt-6">
			<CardHeader>
				<CardTitle class="text-lg">队列说明</CardTitle>
			</CardHeader>
			<CardContent class="space-y-4">
				<div class="grid gap-4 md:grid-cols-2">
					<div>
						<h4 class="font-medium mb-2">任务处理机制</h4>
						<ul class="text-sm text-muted-foreground space-y-1">
							<li>• 扫描期间的操作会自动加入对应队列等待处理</li>
							<li>• 扫描完成后会按顺序自动处理所有队列中的任务</li>
							<li>• 处理顺序：配置 → 删除 → 添加</li>
						</ul>
					</div>
					<div>
						<h4 class="font-medium mb-2">队列状态</h4>
						<div class="space-y-2 text-sm">
							<div class="flex items-center gap-2">
								<Badge variant="outline">空闲</Badge>
								<span class="text-muted-foreground">队列为空且未在处理</span>
							</div>
							<div class="flex items-center gap-2">
								<Badge variant="secondary">等待中</Badge>
								<span class="text-muted-foreground">有任务等待处理</span>
							</div>
							<div class="flex items-center gap-2">
								<Badge variant="destructive">处理中</Badge>
								<span class="text-muted-foreground">正在处理队列中的任务</span>
							</div>
						</div>
					</div>
				</div>
			</CardContent>
		</Card>
	{/if}
</div> 