# bili-sync v2.7.2 Final - 高级功能更新文档

## 📋 概述

bili-sync v2.7.2 Final 带来了革命性的功能更新，包括智能合集管理、全球时区支持、UP主投稿风控系统和完美的移动端适配。本文档详细介绍了所有新功能的使用方法和技术实现。

## 🎯 主要新功能

### 1. 合集管理革命

#### 📺 剧集风格命名系统

**新功能亮点：**
- **S01E01 格式命名**：合集视频现在支持标准剧集命名格式
- **智能集数分配**：基于视频发布时间自动分配集数编号
- **统一/分离双模式**：灵活的文件组织方式

**配置选项：**

```
合集文件夹模式：
├── 分离模式（默认）：每个视频独立文件夹
└── 统一模式：所有视频在合集文件夹下，使用S01E01命名
```

**统一模式示例：**
```
我的合集/
├── S01E01 - 第一个视频标题.mp4
├── S01E02 - 第二个视频标题.mp4
├── S01E03 - 第三个视频标题.mp4
└── ...
```

**分离模式示例：**
```
我的合集/
├── 第一个视频标题/
│   └── 第一个视频标题.mp4
├── 第二个视频标题/
│   └── 第二个视频标题.mp4
└── ...
```

#### 🔄 智能集数分配算法

系统根据视频在合集中的发布时间顺序自动分配集数：

1. **时间排序**：按视频发布时间（pubtime）升序排列
2. **自动编号**：从1开始顺序分配集数
3. **动态更新**：新视频自动获得下一个集数
4. **错误处理**：找不到视频时使用默认命名

**技术实现：**
```rust
// workflow.rs 第1989-2018行
async fn get_collection_video_episode_number(
    connection: &DatabaseConnection,
    collection_id: i32,
    bvid: &str,
) -> Result<i32> {
    // 按发布时间排序获取所有视频
    let videos = video::Entity::find()
        .filter(video::Column::CollectionId.eq(collection_id))
        .order_by_asc(video::Column::Pubtime)
        .select_only()
        .columns([video::Column::Bvid, video::Column::Pubtime])
        .into_tuple::<(String, chrono::NaiveDateTime)>()
        .all(connection)
        .await?;

    // 找到当前视频位置，返回序号（从1开始）
    for (index, (video_bvid, _)) in videos.iter().enumerate() {
        if video_bvid == bvid {
            return Ok((index + 1) as i32);
        }
    }
    
    Err(anyhow!("视频未找到"))
}
```

### 2. 全球时区支持系统

#### 🌍 支持的时区列表

| 时区 | 标识符 | 描述 | UTC偏移 |
|------|--------|------|---------|
| 北京时间 | Asia/Shanghai | 中国标准时间 | UTC+8 |
| 协调世界时 | UTC | 世界标准时间 | UTC+0 |
| 纽约时间 | America/New_York | 美国东部时间 | UTC-5/-4 |
| 洛杉矶时间 | America/Los_Angeles | 美国西部时间 | UTC-8/-7 |
| 伦敦时间 | Europe/London | 英国时间 | UTC+0/+1 |
| 巴黎时间 | Europe/Paris | 欧洲中部时间 | UTC+1/+2 |
| 东京时间 | Asia/Tokyo | 日本标准时间 | UTC+9 |
| 首尔时间 | Asia/Seoul | 韩国标准时间 | UTC+9 |
| 悉尼时间 | Australia/Sydney | 澳洲东部时间 | UTC+10/+11 |
| 迪拜时间 | Asia/Dubai | 阿联酋时间 | UTC+4 |
| 新加坡时间 | Asia/Singapore | 新加坡时间 | UTC+8 |
| 香港时间 | Asia/Hong_Kong | 香港时间 | UTC+8 |
| 台北时间 | Asia/Taipei | 台湾时间 | UTC+8 |

#### 🛠️ 功能特性

**智能时间格式化：**
```typescript
// 支持多种时间格式
formatTimestamp(timestamp, timezone, 'datetime') // 完整日期时间
formatTimestamp(timestamp, timezone, 'date')     // 仅日期
formatTimestamp(timestamp, timezone, 'time')     // 仅时间
```

**相对时间显示：**
- 1分钟内：显示"刚刚"
- 1小时内：显示"X分钟前"
- 1天内：显示"X小时前"
- 1周内：显示"X天前"
- 超过1周：显示具体日期时间

**持久化设置：**
- 设置自动保存到浏览器本地存储
- 跨会话保持用户偏好
- 默认使用Asia/Shanghai时区

#### 📱 时区配置界面

在设置页面的"其他设置"部分：

```
时区设置：[下拉选择框]
├── 北京时间 (UTC+8)     ← 默认选择
├── 协调世界时 (UTC+0)
├── 纽约时间 (UTC-5/-4)
└── ... 其他时区选项

说明：选择时区后，所有时间戳将转换为对应时区显示
```

### 3. UP主投稿智能风控系统

#### 🎯 系统概述

针对大型UP主（700+视频）设计的智能风控系统，通过4层配置策略有效避免B站API限制，确保稳定下载。

#### 🔧 4层配置体系

##### 🎯 基础优化配置（蓝色区域）

| 参数 | 默认值 | 说明 |
|------|--------|------|
| 大量视频UP主阈值 | 100 | 超过此视频数量的UP主将启用风控策略 |
| 基础请求间隔 | 200ms | 每个请求之间的基础延迟时间 |
| 大量视频延迟倍数 | 2倍 | 大量视频UP主的延迟倍数 |
| 最大延迟倍数 | 4倍 | 渐进式延迟的最大倍数限制 |
| 启用渐进式延迟 | ✅ 是 | 随着请求次数增加逐步延长延迟时间 |

##### 📈 增量获取配置（绿色区域）

| 参数 | 默认值 | 说明 |
|------|--------|------|
| 启用增量获取 | ✅ 是 | 优先获取最新视频，减少不必要的请求 |
| 增量获取失败时回退到全量获取 | ✅ 是 | 确保数据完整性 |

**增量获取逻辑：**
```rust
// submission.rs 第42-50行
fn should_take(&self, release_datetime: &chrono::DateTime<Utc>, latest_row_at: &chrono::DateTime<Utc>) -> bool {
    if CONFIG.submission_risk_control.enable_incremental_fetch {
        // 增量模式：只获取比上次扫描时间更新的视频
        let should_take = release_datetime > latest_row_at;
        
        if should_take {
            debug!("增量获取: 发现新视频");
        }
        
        should_take
    } else {
        // 全量模式：获取所有视频
        true
    }
}
```

##### 📦 分批处理配置（紫色区域）

| 参数 | 默认值 | 说明 |
|------|--------|------|
| 启用分批处理 | ❌ 否 | 将大量请求分批处理，降低服务器压力 |
| 分批大小 | 5页 | 每批处理的页数 |
| 批次间延迟 | 2秒 | 每批之间的等待时间 |

##### 🔄 自动退避配置（橙色区域）

| 参数 | 默认值 | 说明 |
|------|--------|------|
| 启用自动退避 | ✅ 是 | 遇到错误时自动增加延迟时间 |
| 自动退避基础时间 | 10秒 | 遇到错误时的基础等待时间 |
| 自动退避最大倍数 | 5倍 | 退避时间的最大倍数限制 |

#### 📊 使用建议

**小型UP主（<100视频）：**
- 使用默认设置即可
- 无需额外配置

**中型UP主（100-500视频）：**
- 启用渐进式延迟和增量获取
- 适当增加基础延迟到300-500ms

**大型UP主（500-1000视频）：**
- 启用分批处理，设置较大的延迟倍数
- 分批大小设为3-5页
- 批次间延迟设为3-5秒

**超大型UP主（>1000视频）：**
- 启用所有风控策略
- 基础延迟提高到500-1000ms
- 延迟倍数设为3-5倍
- 分批大小设为2-3页

**频繁遇到412错误：**
- 增加基础请求间隔到500ms以上
- 提高延迟倍数到5-8倍
- 启用自动退避功能

#### 🔄 自动风控重置

系统检测到风控时自动执行：

1. **重置失败任务**：将状态为失败(3)的任务重置为未开始(0)
2. **重置进行中任务**：将状态为进行中(2)的任务重置为未开始(0)
3. **保留成功任务**：状态为成功(1)的任务保持不变
4. **日志记录**：详细记录重置的视频和页面数量

```rust
// workflow.rs 第1873-1987行
pub async fn auto_reset_risk_control_failures(connection: &DatabaseConnection) -> Result<()> {
    info!("检测到风控，开始自动重置失败、进行中和未完成的下载任务...");
    
    // 重置视频和页面的未完成状态
    // 只重置非完全成功的任务
    // ...
    
    info!("风控自动重置完成：重置了 {} 个视频和 {} 个页面的未完成任务状态", 
          resetted_videos, resetted_pages);
}
```

### 4. 移动端完美适配

#### 📱 响应式设计系统

**断点设置：**
```svelte
// 第149-150行
let innerWidth: number;
let isMobile: boolean = false;
$: isMobile = innerWidth < 768; // md断点
```

**768px以下自动切换移动端布局**

#### 🎨 移动端UI优化

**布局适配：**
```svelte
<!-- 主布局 -->
<div class="flex {isMobile ? 'flex-col' : 'gap-8'}">

<!-- 标题区域 -->
<div class="flex {isMobile ? 'flex-col gap-2' : 'items-center justify-between'}">

<!-- 按钮适配 -->
<Button class={isMobile ? 'w-full' : ''}>
```

**移动端特性：**

1. **全屏按钮**：所有按钮在移动端自动变为全宽度
2. **垂直布局**：表单和帮助面板垂直排列
3. **触摸优化**：增大触摸目标，优化手指操作
4. **滚动优化**：帮助面板支持垂直滚动

**帮助面板移动端适配：**
```svelte
<!-- 帮助面板 -->
{#if showHelp}
    <div class={isMobile ? 'mt-6 w-full' : 'flex-1'}>
        <div class="rounded-lg border bg-white {isMobile ? '' : 'h-full'} 
                    flex flex-col overflow-hidden {isMobile ? '' : 'sticky top-6'} 
                    max-h-[calc(100vh-200px)]">
            <!-- 帮助内容 -->
        </div>
    </div>
{/if}
```

#### 🔧 交互体验优化

**移动端特殊处理：**

1. **拖拽功能**：编解码器优先级排序在移动端禁用
2. **表单验证**：实时验证，减少输入错误
3. **加载状态**：清晰的加载指示器
4. **错误提示**：Toast消息适配移动端显示

**保存按钮区域：**
```svelte
<!-- 提交按钮区域 -->
<div class="flex {isMobile ? 'flex-col' : ''} gap-2 border-t pt-4">
    <Button type="submit" disabled={saving} class={isMobile ? 'w-full' : ''}>
        {saving ? '保存中...' : '保存设置'}
    </Button>
    <Button type="button" variant="outline" onclick={loadConfig} class={isMobile ? 'w-full' : ''}>
        重置
    </Button>
</div>
```

## 🛠️ 技术实现详解

### TypeScript 类型系统

#### 配置接口定义

```typescript
// types.ts 第161-219行
export interface ConfigResponse {
    video_name: string;
    page_name: string;
    multi_page_name?: string;
    bangumi_name?: string;
    folder_structure: string;
    collection_folder_mode?: string;  // 新增：合集文件夹模式
    time_format: string;
    interval: number;
    nfo_time_type: string;
    parallel_download_enabled: boolean;
    parallel_download_threads: number;
    
    // 视频质量设置
    video_max_quality?: string;
    video_min_quality?: string;
    audio_max_quality?: string;
    audio_min_quality?: string;
    codecs?: string[];
    no_dolby_video?: boolean;
    no_dolby_audio?: boolean;
    no_hdr?: boolean;
    no_hires?: boolean;
    
    // 弹幕设置
    danmaku_duration?: number;
    danmaku_font?: string;
    danmaku_font_size?: number;
    danmaku_width_ratio?: number;
    danmaku_horizontal_gap?: number;
    danmaku_lane_size?: number;
    danmaku_float_percentage?: number;
    danmaku_bottom_percentage?: number;
    danmaku_opacity?: number;
    danmaku_bold?: boolean;
    danmaku_outline?: number;
    danmaku_time_offset?: number;
    
    // 并发控制设置
    concurrent_video?: number;
    concurrent_page?: number;
    rate_limit?: number;
    rate_duration?: number;
    
    // 其他设置
    cdn_sorting?: boolean;
    timezone?: string;  // 新增：时区设置
    
    // UP主投稿风控配置（13个参数）
    large_submission_threshold?: number;
    base_request_delay?: number;
    large_submission_delay_multiplier?: number;
    enable_progressive_delay?: boolean;
    max_delay_multiplier?: number;
    enable_incremental_fetch?: boolean;
    incremental_fallback_to_full?: boolean;
    enable_batch_processing?: boolean;
    batch_size?: number;
    batch_delay_seconds?: number;
    enable_auto_backoff?: boolean;
    auto_backoff_base_seconds?: number;
    auto_backoff_max_multiplier?: number;
}
```

### Rust 后端架构

#### 合集特殊处理逻辑

```rust
// workflow.rs 第914-935行
let base_name = if let VideoSourceEnum::Collection(collection_source) = video_source {
    // 合集视频的特殊处理
    let config = crate::config::reload_config();
    if config.collection_folder_mode.as_ref() == "unified" {
        // 统一模式：使用S01E01格式命名
        match get_collection_video_episode_number(connection, collection_source.id, &video_model.bvid).await {
            Ok(episode_number) => {
                format!("S01E{:02} - {}", episode_number, video_model.name)
            }
            Err(_) => {
                // 如果获取序号失败，使用默认命名
                let handlebars = create_handlebars_with_helpers();
                let rendered = handlebars.render_template(&config.page_name, &page_format_args(video_model, &page_model))?;
                crate::utils::filenamify::filenamify(&rendered)
            }
        }
    } else {
        // 分离模式：使用原有逻辑
        let handlebars = create_handlebars_with_helpers();
        let rendered = handlebars.render_template(&config.page_name, &page_format_args(video_model, &page_model))?;
        crate::utils::filenamify::filenamify(&rendered)
    }
}
```

#### 时区工具函数

```typescript
// timezone.ts 完整实现
export function formatTimestamp(
    timestamp: string | number | Date,
    timezone: string = getCurrentTimezone(),
    format: 'datetime' | 'date' | 'time' = 'datetime'
): string {
    try {
        let date: Date;
        
        if (typeof timestamp === 'string') {
            date = new Date(timestamp);
        } else if (typeof timestamp === 'number') {
            date = new Date(timestamp < 1e12 ? timestamp * 1000 : timestamp);
        } else {
            date = timestamp;
        }

        if (isNaN(date.getTime())) {
            return '无效时间';
        }

        const options: Intl.DateTimeFormatOptions = {
            timeZone: timezone,
            year: 'numeric',
            month: '2-digit',
            day: '2-digit',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
            hour12: false
        };

        switch (format) {
            case 'date':
                delete options.hour;
                delete options.minute;
                delete options.second;
                break;
            case 'time':
                delete options.year;
                delete options.month;
                delete options.day;
                break;
        }

        return new Intl.DateTimeFormat('zh-CN', options).format(date);
    } catch (error) {
        console.error('时间格式化失败:', error);
        return '格式化失败';
    }
}
```

## 📚 用户使用指南

### 合集管理最佳实践

#### 选择合适的文件夹模式

**推荐使用分离模式的场景：**
- 视频内容差异较大
- 需要为每个视频单独管理元数据
- 视频数量较少（<50个）
- 希望保持传统的文件夹结构

**推荐使用统一模式的场景：**
- 视频是连续剧集或教程系列
- 视频数量较多（>50个）
- 希望模拟电视剧的文件组织方式
- 使用媒体服务器（如Plex, Emby）管理内容

#### 配置步骤

1. **进入设置页面**
2. **找到"文件命名模板"部分**
3. **选择"合集文件夹模式"**
   - 分离模式：每个视频独立文件夹（默认）
   - 统一模式：所有视频在合集文件夹下
4. **保存设置**

### 时区配置指南

#### 设置时区

1. **打开设置页面**
2. **滚动到"其他设置"部分**
3. **选择"时区设置"下拉菜单**
4. **选择您所在的时区**
5. **点击"保存设置"**

#### 时区选择建议

**中国大陆用户：** Asia/Shanghai (北京时间)  
**中国香港用户：** Asia/Hong_Kong (香港时间)  
**中国台湾用户：** Asia/Taipei (台北时间)  
**美国东部用户：** America/New_York (纽约时间)  
**美国西部用户：** America/Los_Angeles (洛杉矶时间)  
**欧洲用户：** Europe/London (伦敦时间) 或 Europe/Paris (巴黎时间)  
**日本用户：** Asia/Tokyo (东京时间)  
**韩国用户：** Asia/Seoul (首尔时间)  

### UP主投稿风控配置

#### 快速配置向导

**第一步：评估UP主规模**
- 查看UP主的总投稿数量
- 确定是否经常遇到风控（412错误）

**第二步：选择配置策略**

**小型UP主（<100视频）：**
```
基础优化：保持默认设置
增量获取：✅ 启用
分批处理：❌ 禁用
自动退避：✅ 启用
```

**中型UP主（100-500视频）：**
```
大量视频阈值：300
基础请求间隔：300ms
延迟倍数：2-3倍
渐进式延迟：✅ 启用
增量获取：✅ 启用
分批处理：可选启用
```

**大型UP主（500-1000视频）：**
```
大量视频阈值：500
基础请求间隔：500ms
延迟倍数：3-4倍
分批处理：✅ 启用，大小5页
批次延迟：3秒
自动退避：✅ 启用，基础时间15秒
```

**超大型UP主（>1000视频）：**
```
大量视频阈值：1000
基础请求间隔：1000ms
延迟倍数：5倍
分批处理：✅ 启用，大小3页
批次延迟：5秒
自动退避：✅ 启用，基础时间20秒，最大倍数8倍
```

**第三步：监控和调整**
- 观察下载日志中的风控信息
- 根据实际情况微调参数
- 遇到频繁风控时适当增加延迟

### 移动端使用技巧

#### 手机浏览器推荐设置

**Chrome 移动版：**
1. 启用"桌面版网站"以获得完整功能
2. 建议横屏操作以获得更好体验

**Safari 移动版：**
1. 设置页面会自动适配移动端
2. 支持所有核心功能

#### 操作建议

**配置设置：**
- 使用全屏按钮，操作更便捷
- 帮助面板会在下方展开，便于查看
- 支持垂直滚动浏览所有选项

**视频管理：**
- 视频列表支持触摸滚动
- 状态信息清晰显示
- 支持搜索和筛选功能

## 🔧 故障排除

### 常见问题与解决方案

#### 合集视频命名问题

**问题：** 统一模式下视频没有使用S01E01格式命名

**解决方案：**
1. 检查合集文件夹模式是否设置为"统一"
2. 确保数据库中有该合集的视频记录
3. 查看日志中是否有集数分配错误
4. 重新扫描该合集

**问题：** 集数编号顺序不正确

**解决方案：**
1. 集数基于视频发布时间（pubtime）排序
2. 如果发布时间不准确，可能导致编号错误
3. 建议使用分离模式或手动调整文件名

#### 时区显示问题

**问题：** 时间显示不正确

**解决方案：**
1. 检查浏览器时区设置是否正确
2. 确认选择的时区是否为目标时区
3. 刷新页面以应用新的时区设置
4. 清除浏览器缓存重试

**问题：** 时区设置不保存

**解决方案：**
1. 检查浏览器是否禁用了localStorage
2. 确认隐私模式下localStorage可能不工作
3. 尝试使用普通浏览模式

#### 风控配置问题

**问题：** 仍然经常遇到412错误

**解决方案：**
1. 增加基础请求间隔到1000ms以上
2. 启用分批处理，减小批处理大小
3. 增大批次间延迟时间
4. 启用自动退避功能

**问题：** 下载速度过慢

**解决方案：**
1. 适当减少延迟参数
2. 禁用不必要的风控策略
3. 对于小型UP主，使用默认设置即可

#### 移动端显示问题

**问题：** 移动端布局异常

**解决方案：**
1. 确保屏幕宽度检测正常工作
2. 刷新页面重新检测屏幕尺寸
3. 尝试旋转屏幕触发重新布局
4. 清除浏览器缓存

**问题：** 按钮无法点击

**解决方案：**
1. 检查是否有其他元素覆盖按钮
2. 尝试滚动页面确保按钮完全可见
3. 使用更新版本的移动浏览器

## 🚀 性能优化建议

### 系统配置优化

**并发设置推荐：**
```
同时处理视频数：3-5（根据硬件性能）
每个视频并发分页数：2-3
请求频率限制：4-8（根据网络状况）
时间窗口：250-500ms
```

**质量设置平衡：**
- 根据存储空间选择合适的视频质量
- 平衡下载速度和文件大小
- 考虑网络带宽限制

### 网络优化

**CDN 排序：**
- 启用CDN排序优化下载节点
- 可能提升下载速度

**多线程下载：**
- 根据网络带宽启用多线程
- 线程数建议设置为4-8

### 存储优化

**路径规划：**
- 使用SSD存储下载的视频
- 规划合理的目录结构
- 定期清理无效文件

## 📈 版本更新说明

### v2.7.2 Final 新功能

**✨ 主要新增功能：**
1. **合集剧集化管理**：S01E01格式命名，智能集数分配
2. **全球时区支持**：13个主要时区，完整本地化
3. **智能风控系统**：4层配置，13个参数，全面防风控
4. **移动端完美适配**：响应式设计，触摸优化

**🔧 技术改进：**
1. **数据库架构优化**：支持时间排序集数分配
2. **TypeScript 类型完善**：全面类型安全
3. **前端响应式重构**：移动端优先设计
4. **错误处理增强**：智能分类和自动恢复

**🐛 问题修复：**
1. 修复合集视频文件夹组织问题
2. 优化大型UP主下载稳定性
3. 改善移动端交互体验
4. 解决时区显示不一致问题

### 向后兼容性

**数据兼容性：**
- 完全兼容现有数据库结构
- 自动迁移配置文件
- 保持原有API接口

**配置兼容性：**
- 所有新配置项都有合理默认值
- 现有配置无需修改即可正常工作
- 渐进式升级，无需重新配置

## 🤝 贡献指南

### 开发环境设置

**前端开发：**
```bash
cd web
npm install
npm run dev
```

**后端开发：**
```bash
cargo build
cargo run
```

### 代码规范

**TypeScript 规范：**
- 使用严格的类型检查
- 遵循 ESLint 配置
- 优先使用函数式编程风格

**Rust 规范：**
- 遵循 rustfmt 格式化
- 使用 clippy 进行代码检查
- 确保所有测试通过

### 提交规范

**提交信息格式：**
```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**类型说明：**
- `feat`: 新功能
- `fix`: 错误修复
- `docs`: 文档更新
- `style`: 代码格式调整
- `refactor`: 代码重构
- `test`: 测试相关
- `chore`: 构建过程或辅助工具的变动

## 📞 支持与反馈

### 获取帮助

**文档资源：**
- [完整使用文档](./README.md)
- [API 接口文档](./api.md)
- [故障排除指南](./troubleshooting.md)

**社区支持：**
- GitHub Issues：报告问题和建议
- 讨论区：技术交流和经验分享

### 反馈渠道

**问题报告：**
1. 提供详细的操作步骤
2. 包含错误日志和截图
3. 说明系统环境信息

**功能建议：**
1. 描述使用场景和需求
2. 提供具体的实现建议
3. 考虑对现有功能的影响

---

**bili-sync v2.7.2 Final** 为您带来更加智能、便捷、国际化的 B站内容管理体验。无论您是个人用户还是内容创作者，都能在这个版本中找到适合的解决方案。

🎉 **开始使用这些激动人心的新功能吧！**