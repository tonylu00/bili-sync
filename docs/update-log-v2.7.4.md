---
title: "v2.7.4: 智能错误处理与系统稳定性重大升级"
date: 2025-06-29
---

# v2.7.4: 智能错误处理与系统稳定性重大升级

此版本是一个重要的稳定性和用户体验增强版本，引入了革命性的87007充电专享视频自动处理机制，完善了任务队列持久化系统，并新增了失败任务智能筛选功能，为用户提供真正的零干预下载体验。

## 🌟 核心新功能

### 🎯 **87007充电专享视频智能处理** - 零干预体验 ⭐⭐⭐

**彻底解决充电专享视频下载困扰！**

- **自动检测机制**：智能识别87007状态码（充电专享视频）
- **即时自动删除**：检测到充电专享视频后自动创建删除任务，避免重复尝试
- **重复保护**：智能检查防止为同一视频创建多个删除任务
- **日志优化**：针对充电专享视频使用专门的日志级别，减少噪音

**技术实现亮点**：
```rust
// 在workflow.rs中的智能检测逻辑
if error_msg.contains("status code: 87007") {
    warn!("检测到充电专享视频「{}」，将自动删除该视频以避免重复尝试: {:#}", &video_model.name, e);
    let delete_task = DeleteVideoTask {
        video_id: video_model.id,
        task_id: format!("auto_delete_87007_{}", video_model.id),
    };
    // 检查是否已有待处理的删除任务，避免重复创建
    if !VIDEO_DELETE_TASK_QUEUE.has_pending_delete_task(video_model.id, connection).await? {
        VIDEO_DELETE_TASK_QUEUE.enqueue_task(delete_task, connection).await?;
        info!("已为充电专享视频「{}」创建自动删除任务", &video_model.name);
    }
}
```

**应用场景**：
- UP主投稿中的部分充电专享内容
- 合集中混合的免费和付费内容
- 收藏夹中包含的充电专享视频
- 番剧中的会员专享集数

### 🛡️ **任务队列持久化系统升级** - 保证数据一致性

**永不丢失的任务管理！**

- **数据库持久化**：所有任务队列状态保存到SQLite数据库
- **启动恢复机制**：程序重启后自动恢复未完成的任务
- **状态同步**：内存队列与数据库状态实时同步
- **故障恢复**：意外重启后任务无缝继续执行

**技术架构**：
```rust
// 新增的任务队列数据库实体
pub struct TaskQueueEntity {
    pub id: i32,
    pub task_type: TaskType,
    pub task_data: String,     // JSON序列化的任务数据
    pub status: TaskStatus,    // Pending/Completed/Failed
    pub retry_count: i32,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

// 启动时任务恢复函数
pub async fn recover_pending_tasks(connection: &DatabaseConnection) -> Result<(), anyhow::Error> {
    // 自动恢复所有Pending状态的任务到内存队列
}
```

**支持的任务类型**：
- 视频源删除任务（DeleteVideoSource）
- 单个视频删除任务（DeleteVideo）
- 视频源添加任务（AddVideoSource）
- 配置更新任务（UpdateConfig）
- 配置重载任务（ReloadConfig）

### 📊 **失败任务智能筛选** - 快速问题定位

**一键找到所有问题视频！**

- **前端筛选界面**：主页新增"仅显示失败任务"选项
- **智能状态分析**：基于下载状态码判断任务失败原因
- **多层筛选逻辑**：支持视频级和分页级失败筛选
- **实时状态更新**：筛选结果实时反映任务状态变化

**筛选实现**：
```rust
// 失败任务筛选逻辑
pub async fn get_failed_videos_in_current_cycle(
    additional_expr: SimpleExpr,
    connection: &DatabaseConnection,
) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
    // 筛选有可重试失败状态的视频和分页
    let result = all_videos
        .into_iter()
        .filter(|(video_model, pages_model)| {
            let video_status = VideoStatus::from(video_model.download_status);
            let video_should_retry = video_status.should_run().iter().any(|&should_run| should_run);
            
            let pages_should_retry = pages_model.iter().any(|page_model| {
                let page_status = PageStatus::from(page_model.download_status);
                page_status.should_run().iter().any(|&should_run| should_run)
            });
            
            video_should_retry || pages_should_retry
        })
        .collect::<Vec<_>>();
}
```

## 🛠️ 系统架构增强

### 🗄️ 数据库架构扩展

**新增核心数据表**：
```sql
-- 任务队列表
CREATE TABLE task_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_type TEXT NOT NULL,        -- 任务类型枚举
    task_data TEXT NOT NULL,        -- JSON格式的任务数据
    status TEXT NOT NULL,           -- Pending/Completed/Failed
    retry_count INTEGER DEFAULT 0,  -- 重试次数
    created_at DATETIME NOT NULL,   -- 创建时间
    updated_at DATETIME NOT NULL    -- 更新时间
);
```

**已删除视频恢复功能**：
- **智能恢复机制**：支持恢复已软删除的视频
- **状态重置**：恢复时重置下载状态为未开始，强制重新下载
- **路径清理**：清空原有路径，重新生成新的文件路径
- **批量操作**：同时重置视频和所有相关分页的状态

### 🔧 API系统完善

**新增队列管理API**：
```typescript
// 视频删除任务检查
GET /api/videos/{id}/has-pending-delete-task

// 队列状态获取（已有功能增强）
GET /api/queue/status
// 返回结构包含：
interface QueueStatusResponse {
    is_scanning: boolean;
    delete_queue: QueueInfo;
    video_delete_queue: QueueInfo;  // 新增
    add_queue: QueueInfo;
    config_queue: ConfigQueueInfo;
}
```

### 📱 前端界面优化

**失败任务筛选界面**：
- **筛选开关**：主页顶部新增"仅显示失败任务"切换开关
- **状态指示**：筛选激活时显示特殊的UI标识
- **实时更新**：筛选状态与视频列表实时同步
- **用户友好**：清晰的筛选提示和状态反馈

## 🐛 关键修复与优化

### 🔧 核心修复

**1. 87007错误处理优化**
- **问题**：充电专享视频导致下载任务反复失败，产生大量错误日志
- **解决**：自动检测并删除充电专享视频，避免无效重试
- **影响**：显著减少日志噪音，提升系统稳定性

**2. 任务队列稳定性增强**
- **问题**：程序重启后队列任务丢失
- **解决**：实现完整的数据库持久化和恢复机制
- **影响**：保证任务执行的连续性和可靠性

**3. 重复任务防护**
- **问题**：可能为同一视频创建多个删除任务
- **解决**：实现has_pending_delete_task检查机制
- **影响**：避免资源浪费和重复操作

**4. 错误日志级别优化**
- **问题**：已删除视频的错误信息级别过高
- **解决**：对"视频已经被删除"错误使用INFO级别记录
- **影响**：减少误导性错误提示，改善日志可读性

### 🎨 界面体验改进

**筛选功能集成**：
- 无缝集成到现有界面，不破坏原有使用习惯
- 筛选状态持久化，页面刷新后保持筛选状态
- 响应式设计，支持各种屏幕尺寸
- 直观的视觉反馈和状态指示

## 📊 性能与兼容性

### ⚡ 性能提升

**数据库查询优化**：
- 新增索引支持任务队列快速查询
- 优化失败任务筛选的查询逻辑
- 减少不必要的数据库操作

**内存使用优化**：
- 智能的任务队列内存管理
- 及时清理已完成的任务数据
- 优化大量任务时的内存占用

### 🌐 兼容性保证

**完全向后兼容**：
- 现有配置和数据无需修改
- 自动数据库结构迁移
- API接口保持兼容
- 无破坏性功能变更

**跨平台支持**：
- Windows、Linux、macOS全平台测试
- Docker部署完全兼容
- 配置文件格式保持一致

## 🔮 未来展望

### 📈 扩展基础

此次更新为后续功能扩展奠定了坚实基础：

**智能错误处理扩展**：
- 更多错误类型的自动处理
- 基于错误模式的智能重试策略
- 错误统计和分析功能

**任务队列功能增强**：
- 任务优先级管理
- 队列任务手动干预
- 任务执行时间调度

**用户体验持续改进**：
- 更细粒度的筛选选项
- 批量任务操作界面
- 智能任务推荐系统

## 🎯 用户价值体现

### ✨ 零干预体验
- **自动处理充电专享视频**：无需用户手动清理，系统自动识别并处理
- **任务执行保证**：程序重启不影响任务执行，用户无感知恢复
- **问题快速定位**：一键筛选失败任务，快速发现并解决问题

### 🛡️ 系统稳定性
- **数据一致性保证**：任务队列持久化确保数据不丢失
- **错误处理完善**：智能错误分类和处理机制
- **资源利用优化**：避免无效重试，提升系统效率

### 📊 运维友好
- **详细状态监控**：完整的任务队列状态展示
- **日志质量提升**：减少噪音，提高有效信息密度
- **故障恢复能力**：自动恢复和错误处理机制

---

**版本号**: v2.7.4  
**发布日期**: 2025年6月29日  
**更新类型**: 重大功能增强 + 稳定性优化  
**兼容性**: 完全向后兼容  
**推荐指数**: ⭐⭐⭐⭐⭐

## 📋 升级建议

**强烈推荐升级**：此版本引入的智能错误处理和任务队列持久化将显著提升系统稳定性和用户体验，特别适合有大量视频源需要管理的用户。

**升级亮点**：
- 彻底解决充电专享视频下载问题
- 系统重启后任务无缝恢复
- 失败任务快速定位和处理
- 零学习成本，功能自动生效

**注意事项**：
- 首次启动会自动进行数据库结构升级
- 建议在升级前备份重要数据
- 新功能默认启用，无需额外配置