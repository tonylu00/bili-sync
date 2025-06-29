# 队列管理功能说明 (v2.7.4增强版)

## 功能概述

队列管理系统经过v2.7.4版本的重大升级，现已支持**任务持久化存储**，实现了真正的企业级任务管理。系统不仅提供实时队列监控，还能确保程序重启后任务无缝恢复，同时新增了87007充电专享视频自动删除功能。

## 主要特性

### 1. 任务持久化存储 ⭐⭐⭐ (v2.7.4新增)
- **数据库存储**：所有任务状态保存到SQLite数据库 `task_queue` 表
- **启动恢复**：程序重启后自动从数据库恢复未完成的任务
- **状态同步**：内存队列与数据库状态实时双向同步
- **故障恢复**：意外重启、崩溃后任务无缝继续执行

### 2. 87007智能处理队列 ⭐⭐ (v2.7.4新增)
- **自动检测**：智能识别87007状态码（充电专享视频）
- **即时删除**：检测到充电专享视频后自动创建删除任务
- **重复保护**：智能检查防止为同一视频创建多个删除任务
- **专用队列**：独立的视频删除任务队列（VIDEO_DELETE_TASK_QUEUE）

### 3. 实时队列状态监控
- **扫描状态**：显示系统是否正在扫描视频源
- **删除队列**：显示等待处理的视频源删除任务
- **视频删除队列**：显示等待处理的单个视频删除任务 (v2.7.4新增)
- **添加队列**：显示等待处理的视频源添加任务  
- **配置队列**：显示等待处理的配置更新和重载任务

### 4. 增强的队列详情展示
- 每个队列显示任务数量和处理状态
- 列出队列中的具体任务信息（含数据库ID）
- 显示任务创建时间、更新时间和重试次数
- 实时更新队列状态（每5秒自动刷新）
- 任务执行历史记录

### 5. 智能任务处理
- 扫描期间的操作自动加入队列
- 扫描完成后按优先级顺序处理：配置 → 删除 → 视频删除 → 添加
- 避免并发冲突，确保数据一致性
- 任务失败自动重试机制

## 技术实现

### 数据库架构 (v2.7.4新增)
```sql
-- 任务队列表
CREATE TABLE task_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_type TEXT NOT NULL,        -- DeleteVideoSource/DeleteVideo/AddVideoSource/UpdateConfig/ReloadConfig
    task_data TEXT NOT NULL,        -- JSON格式的任务数据
    status TEXT NOT NULL,           -- Pending/Completed/Failed
    retry_count INTEGER DEFAULT 0,  -- 重试次数
    created_at DATETIME NOT NULL,   -- 创建时间
    updated_at DATETIME NOT NULL    -- 更新时间
);
```

### 核心功能实现
```rust
// 启动时任务恢复机制
pub async fn recover_pending_tasks(connection: &DatabaseConnection) -> Result<(), anyhow::Error> {
    // 查询所有Pending状态的任务并恢复到内存队列
    let pending_tasks = TaskQueueEntity::find()
        .filter(task_queue::Column::Status.eq(TaskStatus::Pending))
        .order_by_asc(task_queue::Column::CreatedAt)
        .all(connection)
        .await?;
    
    // 分类恢复到对应的内存队列
    for db_task in pending_tasks {
        match db_task.task_type {
            TaskType::DeleteVideo => { /* 恢复到VIDEO_DELETE_TASK_QUEUE */ }
            TaskType::DeleteVideoSource => { /* 恢复到DELETE_TASK_QUEUE */ }
            // ... 其他任务类型
        }
    }
}

// 87007错误自动处理机制
if error_msg.contains("status code: 87007") {
    warn!("检测到充电专享视频「{}」，将自动删除该视频", &video_model.name);
    let delete_task = DeleteVideoTask {
        video_id: video_model.id,
        task_id: format!("auto_delete_87007_{}", video_model.id),
    };
    VIDEO_DELETE_TASK_QUEUE.enqueue_task(delete_task, connection).await?;
}
```

### 后端 API
- **路由**：`GET /api/queue-status`
- **功能**：获取所有队列的实时状态信息
- **返回数据**：
  - 扫描状态
  - 各队列长度和处理状态
  - 队列中的任务详情
  - 数据库任务统计信息 (v2.7.4新增)

**新增API (v2.7.4)**：
- `GET /api/videos/{id}/has-pending-delete-task` - 检查视频是否有待处理的删除任务
- 队列状态响应包含更多详细信息：任务数据库ID、创建时间、重试次数等

### 前端页面
- **路径**：`/queue`
- **导航**：侧边栏底部"任务队列"菜单
- **功能**：
  - 状态总览卡片
  - 队列详情展示
  - 手动刷新按钮
  - 自动刷新（5秒间隔）

## 用户界面

### 状态总览
- 4个状态卡片显示关键信息
- 颜色编码的状态标识：
  - 🟢 空闲：队列为空且未在处理
  - 🟡 等待中：有任务等待处理
  - 🔴 处理中：正在处理队列中的任务

### 队列详情
- 删除队列：显示等待删除的视频源任务
- 添加队列：显示等待添加的视频源任务
- 配置队列：分别显示更新配置和重载配置任务

### 说明信息
- 任务处理机制说明
- 队列状态含义解释
- 处理顺序说明

## 使用场景

### 日常使用场景
1. **监控系统状态**：实时了解系统是否在扫描，各队列的繁忙程度
2. **操作确认**：确认自己的操作是否已加入队列等待处理
3. **故障排查**：当操作没有立即生效时，可以查看是否在队列中等待
4. **系统管理**：管理员可以监控系统负载和任务处理情况

### v2.7.4新增场景
5. **充电专享视频处理**：系统自动检测并删除87007错误的充电专享视频
6. **任务持续性保证**：程序意外重启后，用户可以看到任务自动恢复
7. **重复操作防护**：避免为同一个视频创建多个删除任务
8. **任务历史追踪**：查看任务的完整生命周期，包括重试次数和状态变更

### 企业级应用场景
9. **批量操作管理**：大量视频源操作时的任务进度监控
10. **灾难恢复**：服务器故障后的任务恢复和继续执行
11. **系统维护**：定期检查任务执行情况和系统健康状态
12. **性能调优**：基于队列数据分析系统瓶颈和优化点

## 注意事项

### 基本使用注意事项
- 队列页面会自动刷新，无需手动刷新
- 扫描期间的所有操作都会被安全地加入队列
- 任务会按照设计的优先级顺序执行，确保系统稳定性
- 队列中的任务信息不包含敏感数据，只显示任务类型和基本信息

### v2.7.4版本特殊注意事项
- **首次启动**：升级到v2.7.4后首次启动会自动创建task_queue表并恢复任务
- **87007处理**：充电专享视频会被自动删除，无需手动干预
- **任务恢复**：程序重启后会自动恢复所有未完成的任务，可能需要几秒钟时间
- **重复保护**：相同的删除任务会被自动去重，不会重复执行
- **数据库大小**：长期运行后task_queue表可能积累大量已完成任务记录，建议定期清理

## 相关文件

### 后端文件
- `crates/bili_sync/src/api/handler.rs` - 队列状态API实现
- `crates/bili_sync/src/task/http_server.rs` - 路由配置
- `crates/bili_sync/src/task/mod.rs` - 任务队列核心实现 (v2.7.4重大更新)
- `crates/bili_sync/src/workflow.rs` - 87007错误处理逻辑 (v2.7.4新增)
- `crates/bili_sync/src/utils/model.rs` - 失败任务筛选逻辑 (v2.7.4新增)

### 数据库文件 (v2.7.4新增)
- `crates/bili_sync_entity/src/task_queue.rs` - 任务队列数据实体
- `crates/bili_sync_migration/src/m20241230_000001_create_task_queue.rs` - 任务队列表迁移

### 前端文件
- `web/src/routes/queue/+page.svelte` - 队列页面组件
- `web/src/routes/+page.svelte` - 主页失败任务筛选 (v2.7.4新增)
- `web/src/lib/types.ts` - 队列相关类型定义 (v2.7.4扩展)
- `web/src/lib/api.ts` - 队列API调用 (v2.7.4增强)
- `web/src/lib/components/app-sidebar.svelte` - 侧边栏导航

## 总结

v2.7.4版本的队列管理系统升级是一次重大的架构改进，它不仅保持了原有的实时监控功能，还增加了企业级的任务持久化能力。87007智能处理功能的加入，真正实现了零干预的用户体验。这些改进让bili-sync从一个简单的下载工具升级为企业级的媒体内容管理系统。

**核心价值**：
- ✅ **零干预体验** - 87007错误自动处理
- ✅ **企业级稳定性** - 任务持久化和故障恢复
- ✅ **完整可视化** - 任务全生命周期监控
- ✅ **智能防护** - 重复操作检测和防止 