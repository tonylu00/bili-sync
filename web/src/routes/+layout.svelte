<script lang="ts">
	import '../app.css';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import SearchBar from '$lib/components/search-bar.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { Button } from '$lib/components/ui/button';
	import { goto } from '$app/navigation';
	import { appStateStore, setQuery, ToQuery } from '$lib/stores/filter';
	import { Toaster } from '$lib/components/ui/sonner/index.js';
	import { breadcrumbStore } from '$lib/stores/breadcrumb';
	import BreadCrumb from '$lib/components/bread-crumb.svelte';
	import { videoSourceStore, setVideoSources } from '$lib/stores/video-source';
	import { onMount } from 'svelte';
	import api from '$lib/api';
	import { toast } from 'svelte-sonner';
	import type { ApiError } from '$lib/types';
	import { page } from '$app/stores';
	import { Plus, Settings, LogOut } from '@lucide/svelte';

	let dataLoaded = false;
	let isAuthenticated = false;

	async function handleSearch(query: string) {
		setQuery(query);
		goto(`/${ToQuery($appStateStore)}`);
	}

	// 退出登录
	function handleLogout() {
		api.setAuthToken('');
		isAuthenticated = false;
		goto('/');
		window.location.reload(); // 重新加载页面以清除状态
	}

	// 初始化共用数据
	onMount(async () => {
		// 检查认证状态
		const token = localStorage.getItem('auth_token');
		if (token) {
			isAuthenticated = true;

			// 初始化视频源数据，所有组件都会用到
			if (!$videoSourceStore) {
				try {
					const response = await api.getVideoSources();
					setVideoSources(response.data);
				} catch (error) {
					console.error('加载视频来源失败:', error);
					if ((error as ApiError).status === 401) {
						// Token 无效
						isAuthenticated = false;
						api.setAuthToken('');
					} else {
						toast.error('加载视频来源失败', {
							description: (error as ApiError).message
						});
					}
				}
			}
		}
		dataLoaded = true;
	});

	// 从全局状态获取当前查询值
	$: searchValue = $appStateStore.query;
	// 判断是否在主页
	$: isHomePage = $page.route.id === '/';
</script>

<Toaster />

<Sidebar.Provider>
	<div class="flex h-screen w-full overflow-hidden">
		{#if isAuthenticated}
			<div data-sidebar="sidebar">
				<AppSidebar />
			</div>
		{/if}
		<Sidebar.Inset class="flex h-screen flex-1 flex-col overflow-hidden">
			{#if isAuthenticated}
				<div
					class="bg-background/95 supports-[backdrop-filter]:bg-background/60 sticky top-0 z-50 flex min-h-[73px] w-full items-center border-b backdrop-blur"
				>
					<div class="flex w-full items-center gap-2 px-4 py-2 sm:gap-4 sm:px-6">
						<Sidebar.Trigger class="shrink-0" data-sidebar="trigger" />
						<div class="min-w-0 flex-1">
							<SearchBar onSearch={handleSearch} value={searchValue} />
						</div>
						<div class="flex items-center gap-1 sm:gap-2">
							{#if isHomePage}
								<Button
									size="sm"
									variant="outline"
									onclick={() => goto('/add-source')}
									class="hidden items-center gap-2 sm:flex"
								>
									<Plus class="h-4 w-4" />
									添加视频源
								</Button>
								<!-- 移动端只显示图标 -->
								<Button
									size="sm"
									variant="outline"
									onclick={() => goto('/add-source')}
									class="flex items-center justify-center sm:hidden"
									title="添加视频源"
								>
									<Plus class="h-4 w-4" />
								</Button>
							{/if}
							<Button
								size="sm"
								variant="outline"
								onclick={() => goto('/settings')}
								class="hidden items-center gap-2 sm:flex"
							>
								<Settings class="h-4 w-4" />
								配置
							</Button>
							<!-- 移动端只显示图标 -->
							<Button
								size="sm"
								variant="outline"
								onclick={() => goto('/settings')}
								class="flex items-center justify-center sm:hidden"
								title="配置"
							>
								<Settings class="h-4 w-4" />
							</Button>
							<Button
								size="sm"
								variant="outline"
								onclick={handleLogout}
								class="hidden items-center gap-2 sm:flex"
							>
								<LogOut class="h-4 w-4" />
								退出
							</Button>
							<!-- 移动端只显示图标 -->
							<Button
								size="sm"
								variant="outline"
								onclick={handleLogout}
								class="flex items-center justify-center sm:hidden"
								title="退出"
							>
								<LogOut class="h-4 w-4" />
							</Button>
						</div>
					</div>
				</div>
			{/if}
			<div class="bg-background flex-1 overflow-auto">
				<div class="w-full px-6 py-6">
					{#if isAuthenticated && $breadcrumbStore.length > 0}
						<div class="mb-6">
							<BreadCrumb items={$breadcrumbStore} />
						</div>
					{/if}
					{#if dataLoaded}
						<slot />
					{/if}
				</div>
			</div>
		</Sidebar.Inset>
	</div>
</Sidebar.Provider>
