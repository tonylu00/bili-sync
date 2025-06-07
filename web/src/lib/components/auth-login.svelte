<script lang="ts">
	import { Input } from './ui/input';
	import { Button } from './ui/button';
	import { createEventDispatcher } from 'svelte';
	import api from '$lib/api';
	import { toast } from 'svelte-sonner';

	const dispatch = createEventDispatcher();

	let authToken = '';
	let authError = '';
	let isVerifying = false;

	// 验证Token的函数
	async function verifyToken() {
		if (!authToken.trim()) {
			authError = '请输入API Token';
			return;
		}

		isVerifying = true;
		authError = '';

		try {
			// 设置 Token
			api.setAuthToken(authToken);

			// 尝试调用一个需要认证的API来验证Token
			await api.getVideoSources();

			// 如果成功，说明Token正确
			dispatch('login-success', { token: authToken });
			toast.success('登录成功');
		} catch (error: any) {
			// 清除无效的 Token
			api.setAuthToken('');

			if (error.status === 401) {
				authError = 'API Token错误，请检查后重试';
			} else {
				authError = '验证失败，请检查网络连接或Token是否正确';
			}
			console.error('Token验证失败:', error);
		} finally {
			isVerifying = false;
		}
	}

	// 处理Enter键登录
	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			verifyToken();
		}
	}
</script>

<div class="flex min-h-screen items-center justify-center bg-gray-50">
	<div class="w-full max-w-md space-y-8">
		<div class="text-center">
			<h1 class="mb-2 text-3xl font-bold text-gray-900">bili-sync 管理页</h1>
			<p class="text-gray-600">请输入API Token以访问管理功能</p>
		</div>
		<div class="rounded-lg bg-white p-8 shadow-md">
			<div class="space-y-4">
				<div>
					<label for="token" class="mb-2 block text-sm font-medium text-gray-700">
						API Token
					</label>
					<Input
						id="token"
						type="password"
						placeholder="请输入API Token"
						bind:value={authToken}
						onkeydown={handleKeyDown}
						class="w-full"
						disabled={isVerifying}
					/>
					{#if authError}
						<p class="mt-2 text-sm text-red-600">{authError}</p>
					{/if}
				</div>
				<Button onclick={verifyToken} disabled={isVerifying || !authToken.trim()} class="w-full">
					{isVerifying ? '验证中...' : '登录'}
				</Button>
			</div>
			<div class="mt-6 text-sm text-gray-500">
				<p>提示：API Token 在配置文件的 auth_token 字段中设置</p>
			</div>
		</div>
	</div>
</div>
