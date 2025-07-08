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
	import ResponsiveButton from '$lib/components/responsive-button.svelte';

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
	<div class="flex h-screen w-full overflow-hidden prevent-horizontal-scroll">
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
								<ResponsiveButton
									size="sm"
									variant="outline"
									onclick={() => goto('/add-source')}
									icon={Plus}
									text="添加视频源"
									title="添加视频源"
								/>
							{/if}
							<ResponsiveButton
								size="sm"
								variant="outline"
								onclick={() => goto('/settings')}
								icon={Settings}
								text="配置"
								title="配置"
							/>
							<ResponsiveButton
								size="sm"
								variant="outline"
								onclick={handleLogout}
								icon={LogOut}
								text="退出"
								title="退出"
							/>
						</div>
					</div>
				</div>
			{/if}
			<div class="bg-background flex-1 overflow-auto smooth-scroll">
				<div class="w-full px-4 py-4 sm:px-6 sm:py-6">
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
