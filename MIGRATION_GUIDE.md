# 配置迁移指南 - 视频源移动到数据库

## 重要变更说明

从此版本开始，**所有视频源配置已从配置文件移动到数据库中**，并通过Web管理界面进行管理。

## 主要变更

### 移除的配置项

以下配置项已从 `config.toml` 中移除：

- `favorite_list` - 收藏夹配置
- `favorite_list_v2` - 收藏夹配置（新版）
- `collection_list` - 合集配置  
- `collection_list_v2` - 合集配置（新版）
- `submission_list` - UP主投稿配置
- `submission_list_v2` - UP主投稿配置（新版）
- `watch_later` - 稍后再看配置
- `bangumi` - 番剧配置

### 保留的配置项

配置文件现在主要用于：

- 服务器设置（`bind_address`, `interval`）
- 文件命名模板（`video_name`, `page_name`, `bangumi_name` 等）
- 下载参数（`concurrent_limit`, `rate_limit`）
- 登录凭据（`credential`）
- 过滤选项（`filter_option`, `danmaku_option`）

## 迁移步骤

### 1. 备份现有配置

```bash
cp config.toml config.toml.backup
```

### 2. 更新配置文件

移除所有视频源相关的配置项，保留其他设置。参考 `example_config.toml` 文件。

### 3. 通过Web界面添加视频源

1. 启动程序：`./bili-sync-rs`
2. 访问Web管理界面：`http://127.0.0.1:12345`
3. 在"视频源管理"页面添加你的视频源：
   - 收藏夹：输入收藏夹ID和保存路径
   - 合集：输入UP主ID、合集ID和保存路径
   - UP主投稿：输入UP主ID和保存路径
   - 稍后再看：输入保存路径
   - 番剧：输入season_id和保存路径

## 优势

### 1. 更好的用户体验
- 图形化界面管理视频源
- 实时添加/删除/修改视频源
- 无需手动编辑配置文件

### 2. 更强的功能
- 支持番剧季度选择
- 视频源状态监控
- 更详细的错误信息

### 3. 更好的维护性
- 配置和数据分离
- 支持数据库备份和恢复
- 更容易的程序升级

## 常见问题

### Q: 我的旧配置会丢失吗？
A: 不会。程序会在首次启动时尝试从旧配置文件迁移视频源到数据库。但建议手动通过Web界面重新添加以确保正确性。

### Q: 可以同时使用配置文件和Web界面管理视频源吗？
A: 不可以。现在只支持通过Web界面管理视频源，配置文件中的视频源设置会被忽略。

### Q: 如何备份我的视频源设置？
A: 视频源现在存储在SQLite数据库中（通常在 `data/bili_sync.db`），备份这个文件即可。

### Q: Web界面无法访问怎么办？
A: 检查 `bind_address` 配置，确保端口没有被占用，防火墙允许访问。

## 技术细节

### 数据库表结构

视频源现在存储在以下数据库表中：
- `favorite` - 收藏夹
- `collection` - 合集
- `submission` - UP主投稿  
- `watch_later` - 稍后再看
- `video_source` - 番剧

### API端点

- `GET /api/video-sources` - 获取所有视频源
- `POST /api/video-sources` - 添加视频源
- `DELETE /api/video-sources/{type}/{id}` - 删除视频源

## 回滚方案

如果需要回滚到旧版本：

1. 恢复备份的配置文件：`cp config.toml.backup config.toml`
2. 使用旧版本的程序
3. 注意：数据库中的视频源数据需要手动转换回配置文件格式

---

如有问题，请查看日志文件或提交Issue。 