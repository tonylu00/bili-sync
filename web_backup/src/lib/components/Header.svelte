<script lang="ts">
	// 移除不必要的API Token管理代码，因为已经在主页面处理了
</script>

<header class="flex items-center justify-between bg-gray-100 p-4">
	<h1 class="text-xl font-bold">bili-sync 管理页</h1>
	<div class="flex items-center space-x-4">
		<!-- actions slot用于显示操作按钮 -->
		<slot name="actions" />
	</div>
</header>

<slot />
