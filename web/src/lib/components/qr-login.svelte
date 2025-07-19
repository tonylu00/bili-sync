<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { Button } from '$lib/components/ui/button';
  import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import { Badge } from '$lib/components/ui/badge';
  // @ts-ignore
  import QRCode from 'qrcode';
  
  export let onLoginSuccess: (userInfo: any) => void = () => {};
  export let onLoginError: (error: string) => void = () => {};
  
  let qrCodeDataUrl = '';
  let status: 'idle' | 'loading' | 'waiting' | 'scanned' | 'expired' | 'error' | 'success' = 'idle';
  let statusMessage = '';
  let sessionId = '';
  let pollInterval: number | null = null;
  let isGenerating = false;
  
  interface QRResponse {
    session_id: string;
    qr_url: string;
    expires_in: number;
  }
  
  interface PollResponse {
    status: string;
    message: string;
    user_info?: {
      user_id: string;
      username: string;
      avatar_url: string;
    };
  }
  
  async function generateQRCode() {
    if (isGenerating) {
      console.log('正在生成中，跳过重复请求');
      return;
    }
    
    try {
      isGenerating = true;
      status = 'loading';
      statusMessage = '正在生成二维码...';
      
      console.log('开始请求生成二维码...');
      console.log('请求URL:', '/api/auth/qr/generate');
      console.log('请求方法:', 'POST');
      console.log('请求体:', JSON.stringify({ client_type: 'web' }));
      
      const response = await fetch('/api/auth/qr/generate', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ client_type: 'web' }),
      });
      
      console.log('生成二维码响应状态:', response.status);
      
      if (!response.ok) {
        const errorText = await response.text();
        console.error('生成二维码失败，响应内容:', errorText);
        throw new Error(errorText || `HTTP ${response.status}`);
      }
      
      const result = await response.json();
      console.log('生成二维码响应数据:', result);
      
      // 适配 ApiResponse 格式
      if (result.status_code !== 200) {
        console.error('生成二维码业务失败，状态码:', result.status_code);
        throw new Error(result.data?.message || '生成二维码失败');
      }
      
      const data: QRResponse = result.data;
      console.log('提取的二维码数据:', data);
      
      sessionId = data.session_id;
      console.log('会话ID:', sessionId);
      console.log('二维码URL:', data.qr_url);
      
      // 使用qrcode库生成二维码
      try {
        qrCodeDataUrl = await QRCode.toDataURL(data.qr_url, {
          width: 256,
          margin: 2,
          color: {
            dark: '#000000',
            light: '#FFFFFF'
          }
        });
        console.log('二维码生成成功');
      } catch (err) {
        console.error('生成二维码图片失败:', err);
        throw new Error('生成二维码图片失败');
      }
      
      status = 'waiting';
      statusMessage = '请使用哔哩哔哩手机客户端扫描二维码';
      
      // 开始轮询状态
      startPolling();
      
    } catch (error: any) {
      console.error('生成二维码异常:', error);
      status = 'error';
      statusMessage = `生成二维码失败: ${error.message}`;
      toast.error(error.message);
      onLoginError(error.message);
    } finally {
      isGenerating = false;
    }
  }
  
  function startPolling() {
    pollInterval = window.setInterval(async () => {
      try {
        const response = await fetch(`/api/auth/qr/poll?session_id=${sessionId}`);
        
        if (!response.ok) {
          const error = await response.text();
          throw new Error(error || `HTTP ${response.status}`);
        }
        
        const result = await response.json();
        
        // 适配 ApiResponse 格式
        if (result.status_code !== 200) {
          throw new Error(result.data?.message || '轮询状态失败');
        }
        
        const data: PollResponse = result.data;
        
        switch (data.status) {
          case 'pending':
            status = 'waiting';
            statusMessage = '等待扫码...';
            break;
            
          case 'scanned':
            status = 'scanned';
            statusMessage = '扫码成功，请在手机上确认登录';
            break;
            
          case 'confirmed':
            status = 'success';
            statusMessage = '登录成功！';
            stopPolling();
            toast.success('登录成功！');
            onLoginSuccess(data.user_info);
            break;
            
          case 'expired':
            status = 'expired';
            statusMessage = '二维码已过期，请重新生成';
            stopPolling();
            break;
            
          default:
            status = 'error';
            statusMessage = data.message || '未知错误';
            stopPolling();
            onLoginError(data.message);
        }
        
      } catch (error: any) {
        console.error('轮询失败:', error);
        // 轮询失败不停止，继续尝试
      }
    }, 3000); // 每3秒轮询一次
  }
  
  function stopPolling() {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }
  
  function resetQRCode() {
    stopPolling();
    qrCodeDataUrl = '';
    sessionId = '';
    status = 'idle';
    statusMessage = '';
    generateQRCode();
  }
  
  onMount(() => {
    if (status === 'idle') {
      generateQRCode();
    }
  });
  
  onDestroy(() => {
    stopPolling();
  });
</script>

<Card class="w-full max-w-md mx-auto">
  <CardHeader class="text-center">
    <CardTitle>扫码登录哔哩哔哩</CardTitle>
    <CardDescription>使用手机客户端扫描二维码快速登录</CardDescription>
  </CardHeader>
  
  <CardContent class="flex flex-col items-center space-y-4">
    {#if status === 'loading'}
      <div class="w-64 h-64 flex items-center justify-center">
        <Skeleton class="w-full h-full" />
      </div>
      <p class="text-sm text-muted-foreground">{statusMessage}</p>
      
    {:else if status === 'error'}
      <div class="w-64 h-64 flex flex-col items-center justify-center space-y-4">
        <div class="text-6xl">⚠️</div>
        <p class="text-sm text-red-600 text-center">{statusMessage}</p>
        <Button on:click={resetQRCode} variant="default">重新生成</Button>
      </div>
      
    {:else if status === 'expired'}
      <div class="w-64 h-64 flex flex-col items-center justify-center space-y-4">
        <div class="text-6xl">⏱️</div>
        <p class="text-sm text-yellow-600 text-center">{statusMessage}</p>
        <Button on:click={resetQRCode} variant="default">重新生成</Button>
      </div>
      
    {:else if status === 'success'}
      <div class="w-64 h-64 flex flex-col items-center justify-center space-y-4">
        <div class="text-6xl text-green-500">✓</div>
        <p class="text-sm text-green-600 text-center font-medium">{statusMessage}</p>
      </div>
      
    {:else if qrCodeDataUrl}
      <div class="relative">
        <img 
          src={qrCodeDataUrl} 
          alt="登录二维码" 
          class="w-64 h-64 border rounded {status === 'scanned' ? 'opacity-75' : ''}"
        />
        
        {#if status === 'scanned'}
          <div class="absolute inset-0 flex flex-col items-center justify-center bg-white/90 rounded">
            <div class="text-green-500 text-4xl mb-2">✓</div>
            <p class="text-green-600 font-medium">已扫描</p>
          </div>
        {/if}
      </div>
      
      <div class="flex items-center space-x-2">
        {#if status === 'waiting'}
          <Badge variant="secondary" class="animate-pulse">
            等待扫码
          </Badge>
        {:else if status === 'scanned'}
          <Badge variant="default" class="bg-green-500">
            已扫描
          </Badge>
        {/if}
        
        <span class="text-sm text-muted-foreground">{statusMessage}</span>
      </div>
    {/if}
    
    <details class="w-full text-sm text-muted-foreground">
      <summary class="cursor-pointer hover:text-foreground">使用帮助</summary>
      <ol class="mt-2 space-y-1 list-decimal list-inside">
        <li>使用手机打开哔哩哔哩客户端</li>
        <li>点击右下角"我的"</li>
        <li>点击右上角扫一扫图标</li>
        <li>扫描上方二维码</li>
        <li>在手机上确认登录</li>
      </ol>
    </details>
  </CardContent>
</Card>