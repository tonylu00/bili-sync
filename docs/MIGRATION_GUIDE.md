# 配置迁移指南 - v2.7.3 升级指南

## ✅ **配置文件兼容性已修复！**

### 🛡️ **问题说明与修复**

**重要更新**：早期版本确实存在配置文件兼容性问题，但现在已经完全修复！

#### ❌ **之前存在的问题**
- 新版本新增了 `folder_structure` 字段但没有设置默认值
- 导致旧配置文件加载时出现 `missing field 'folder_structure'` 错误
- 这违背了我们的向后兼容承诺

#### ✅ **已修复的解决方案**
```rust
// 新增默认值函数
fn default_folder_structure() -> Cow<'static, str> {
    Cow::Borrowed("Season 1")
}

// 为字段添加默认值属性
#[serde(default = "default_folder_structure")]
pub folder_structure: Cow<'static, str>,
```

#### 🔧 **现在的兼容性保护**

1. **完全的默认值覆盖**
- 所有新增字段都有 `#[serde(default)]` 属性
- 旧配置文件中缺少的字段会自动使用默认值
- Rust 的 `serde` 库完全支持这种向后兼容

2. **字段兼容性映射**

| 配置字段 | 旧版本 | 新版本处理 | 默认值 |
|---------|--------|------------|--------|
| `folder_structure` | ❌ 不存在 | ✅ 自动添加 | `"Season 1"` |
| `multi_page_name` | ❌ 不存在 | ✅ 自动添加 | `"{{title}}-P{{pid_pad}}"` |
| `bangumi_name` | ❌ 不存在 | ✅ 自动添加 | `"S{{season_pad}}E{{pid_pad}}-{{pid_pad}}"` |
| `favorite_list` | ✅ 存在 | ✅ 被忽略 | N/A |
| `collection_list` | ✅ 存在 | ✅ 被忽略 | N/A |
| `submission_list` | ✅ 存在 | ✅ 被忽略 | N/A |
| 其他共同字段 | ✅ 存在 | ✅ 正常解析 | N/A |

### 📝 **现在的升级过程**

```log
# 使用旧配置文件启动新版本：
INFO 欢迎使用 Bili-Sync，当前程序版本：2.6.2
INFO 开始加载配置文件..
INFO 配置文件加载完毕，覆盖刷新原有配置  # 自动添加缺失字段
INFO 检查配置文件..
INFO 您可以访问管理页 http://0.0.0.0:12345/ 添加视频源  # 正常启动
```

### 🎯 **测试结果**

✅ **旧配置文件现在可以正常加载**
✅ **程序正常启动，无任何错误**
✅ **所有新字段自动添加默认值**
✅ **旧的视频源配置被安全忽略**

## 🆕 v2.7.3 重大更新

### 🔥 主要新特性

1. **配置热重载系统**
   - 配置完全基于数据库存储
   - 支持实时热重载，无需重启
   - 通过任务队列处理，避免数据库锁定

2. **文件名处理增强**
   - 增强的 filenamify 函数
   - 自动处理所有特殊字符
   - 全角字符智能转换

3. **初始设置向导**
   - 首次启动显示友好的设置界面
   - 简化 Cookie 配置流程

4. **优化与修复**
   - 修复内存泄漏问题
   - 修复凭证保存问题
   - 支持受限模式运行

## 重要变更说明

从 v2.6.2 开始，**所有视频源配置已从配置文件移动到数据库中**，并通过Web管理界面进行管理。v2.7.3 进一步增强了配置系统。

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

## 🔧 **程序升级数据库保护机制**

### ✅ **自动迁移系统**

bili-sync具有完善的数据库自动迁移系统，**程序升级后绝对不会导致重复下载**：

#### 🎯 **SeaORM Migration 框架**
- 使用专业的数据库迁移管理框架
- 每次程序启动时自动检测并应用必要的数据库结构更新
- 确保向后兼容和数据完整性

#### 🛡️ **下载状态完全保留**
```rust
// 所有迁移都会保留关键的 download_status 字段
video::Column::DownloadStatus  // 永远不会被清空或重置
page::Column::DownloadStatus   // 保持所有分页的下载状态
```

#### 📊 **版本化迁移管理**
当前迁移版本包括：
- `m20240322_000001_create_table` - 初始数据库结构
- `m20240505_130850_add_collection` - 添加合集支持
- `m20240709_130914_watch_later` - 稍后再看功能
- `m20240724_161008_submission` - UP主投稿支持
- `m20250122_062926_add_latest_row_at` - 增量同步优化
- `m20250519_000001_add_source_id` - 统一视频源管理
- `m20250525_000002_add_season_number` - 番剧季度支持
- `m20250531_000001_fix_fid_type` - 修复收藏夹ID类型
- `m20250601_000001_fix_compatibility` - **兼容性修复**

#### 🔍 **智能兼容性检测**
最新的兼容性迁移(`m20250601_000001_fix_compatibility`)专门处理升级场景：

```sql
-- 自动检测缺失的列并添加
ALTER TABLE favorite ADD COLUMN latest_row_at TIMESTAMP DEFAULT '1970-01-01 00:00:00'

-- 智能更新时间戳，保持已下载视频的历史记录
UPDATE favorite SET latest_row_at = (
    SELECT IFNULL(MAX(favtime), '1970-01-01 00:00:00') 
    FROM video WHERE favorite_id = favorite.id
) WHERE latest_row_at = '1970-01-01 00:00:00'
```

### 📋 **升级过程详解**

#### 🚀 **程序启动时的迁移流程**
```log
INFO 检测到现有数据库文件，将在必要时应用迁移
INFO SQLite WAL 模式已启用，性能优化参数已应用
```

1. **数据库连接** - 连接到现有数据库文件
2. **迁移检测** - 自动检测需要应用的迁移
3. **结构更新** - 安全地添加新列和索引
4. **数据保护** - 所有现有数据保持不变
5. **状态校验** - 确保下载状态完整性

#### 🛡️ **下载状态保护策略**

1. **永不重置**：所有已完成的下载状态永远不会被清空
2. **字段新增**：只添加新字段，从不删除关键数据
3. **向后兼容**：新功能不影响已有视频的状态
4. **冲突处理**：使用 `OnConflict::do_nothing()` 避免重复创建

#### 🔧 **技术实现细节**

```rust
// 创建视频记录时的冲突处理
video::Entity::insert_many(video_models)
    .on_conflict(OnConflict::new().do_nothing().to_owned())
    .do_nothing()  // 如果视频已存在，什么都不做
    .exec(connection)
    .await?;

// 筛选未完成的视频时的状态检查
video::Entity::find()
    .filter(
        video::Column::Valid.eq(true)
        .and(video::Column::DownloadStatus.lt(STATUS_COMPLETED))  // 只处理未完成的
    )
```

### 🎯 **具体保护示例**

#### 升级前后对比
```log
# 升级前的数据库
Video ID: 123, BVID: BV1xx, download_status: 2147483655 (已完成)
Video ID: 124, BVID: BV2xx, download_status: 1234567890 (部分完成)

# 程序升级并应用迁移后
Video ID: 123, BVID: BV1xx, download_status: 2147483655 (保持不变-已完成)
Video ID: 124, BVID: BV2xx, download_status: 1234567890 (保持不变-部分完成)
# 新增字段: season_number, episode_number, source_id 等 (NULL值)
```

#### 重新扫描时的行为
```log
INFO 开始下载「收藏夹名称」视频..
INFO 找到 1 个未处理完成的视频  # 只有ID 124会被重新处理
DEBUG 处理视频「BV1xx」封面已成功过，跳过  # ID 123被自动跳过
INFO 处理视频「BV2xx」封面成功  # 继续处理未完成的部分
```

## 📋 **重复下载说明**

### ✅ **不会重复下载**

**重新添加视频源不会导致重复下载**，因为bili-sync具有完善的视频去重机制：

#### 🎯 **数据库级别去重**
- 每个视频在数据库中都有唯一的标识（BVID）
- 重新添加已存在的视频源时，系统只会更新视频源配置，不会重复创建视频记录

#### 🔍 **下载状态检测**
- 系统通过 `download_status` 字段精确跟踪每个视频的下载状态
- 只有 `download_status < STATUS_COMPLETED` 的视频才会被重新下载
- 已完成下载的视频会被自动跳过

#### 📊 **多层级状态管理**
```
视频下载状态包含5个子任务：
- 视频封面 (状态值: 0-7)
- 视频信息 (状态值: 0-7) 
- UP主头像 (状态值: 0-7)
- UP主信息 (状态值: 0-7)
- 分P下载 (状态值: 0-7)
```

当所有子任务状态为7（完成）时，整个视频被标记为已完成，不会重复下载。

#### 🛡️ **文件存在性检查**
- 系统会检查本地文件是否已存在
- 已存在的文件会被跳过，显示 "已成功过，跳过" 的日志

### 📝 **实际表现**

当你重新添加之前的视频源时：

```log
INFO 开始下载「收藏夹名称」视频..
INFO 找到 0 个未处理完成的视频  # 已下载的视频不会被计入
INFO 下载「收藏夹名称」视频完成
```

或者如果有新视频：

```log
INFO 开始下载「收藏夹名称」视频..
INFO 找到 5 个未处理完成的视频  # 只处理新增的视频
DEBUG 处理视频「已下载视频」封面已成功过，跳过
DEBUG 处理视频「已下载视频」详情已成功过，跳过
INFO 处理视频「新视频」封面成功
```

## 🚀 v2.7.3 升级步骤

### 1. 备份现有数据

```bash
# 备份配置文件
cp config.toml config.toml.backup

# 备份数据库（如果存在）
cp data.sqlite data.sqlite.backup
```

### 2. 下载新版本

从 [发布页面](https://github.com/qq1582185982/bili-sync-01/releases) 下载 v2.7.3 版本。

### 3. 首次启动

```bash
./bili-sync-rs
```

首次启动时：
- 如果未设置凭据，会显示初始设置向导
- 系统会自动迁移现有配置到数据库
- 所有已下载的视频状态会保留

### 4. 配置管理

访问 Web 管理界面：`http://127.0.0.1:12345`

#### 新的配置管理方式
- **设置页面**：管理所有系统配置
- **配置热重载**：修改后立即生效
- **配置历史**：查看配置变更记录

### 5. 视频源管理

在"添加视频源"页面重新添加你的视频源：
- 收藏夹：输入收藏夹ID和保存路径
- 合集：输入UP主ID、合集ID和保存路径
- UP主投稿：输入UP主ID和保存路径
- 稍后再看：输入保存路径
- 番剧：输入season_id和保存路径

> [!TIP]
> 重新添加视频源不会导致重复下载，系统会智能识别已下载的视频。

## 🌟 v2.7.3 新增优势

### 1. 配置热重载
- **实时生效**：配置修改后无需重启
- **原子操作**：使用 ArcSwap 确保线程安全
- **历史记录**：所有配置变更可追溯

### 2. 文件名智能处理
- **全角字符转换**：`：` → `-`，`「」` → `[]`
- **特殊符号处理**：自动替换 Windows 不支持的字符
- **长度限制**：智能截断过长文件名
- **UTF-8 安全**：确保不会在字符边界截断

### 3. 任务队列优化
- **避免锁定**：配置保存通过队列处理
- **并发安全**：多个操作不会冲突
- **错误恢复**：失败任务自动重试

### 4. 初始设置体验
- **向导式配置**：首次启动引导设置
- **受限模式**：未设置凭据时可浏览但不下载
- **智能提示**：实时验证配置有效性

### 5. 原有优势保留
- 智能去重保护
- 自动数据库迁移
- 图形化管理界面
- 番剧季度选择

## ❓ 常见问题

### Q: 从旧版本升级需要注意什么？
A: v2.7.3 完全向后兼容，主要注意：
- 首次启动会自动迁移配置到数据库
- 所有已下载视频状态会保留
- 视频源需要通过 Web 界面重新添加

### Q: 配置文件还需要吗？
A: config.toml 仅作为初始配置，实际配置存储在数据库中。升级后可以保留作为参考。

### Q: 文件名包含特殊字符会出错吗？
A: 不会！v2.7.3 的 filenamify 函数会自动处理所有特殊字符：
- 全角字符自动转换
- Windows 保留字符替换
- 长度自动限制

### Q: 配置修改需要重启吗？
A: 不需要！v2.7.3 支持配置热重载，修改后立即生效。

### Q: 重新添加视频源会重复下载视频吗？
A: **不会！** bili-sync具有完善的去重机制：
- 数据库级别的视频去重（基于BVID）
- 精确的下载状态跟踪
- 智能的文件存在性检查
- 已下载的视频会自动跳过

### Q: 程序升级后会重复下载所有视频吗？
A: **绝对不会！** bili-sync具有专业的数据库迁移系统：
- 自动检测和应用数据库结构更新
- 完全保留所有已下载视频的状态
- 智能兼容性修复确保向后兼容
- 只有真正未完成的视频才会被重新处理

### Q: 如何确认升级后数据是否安全？
A: 查看程序启动日志：
```log
INFO 检测到现有数据库文件，将在必要时应用迁移
INFO SQLite WAL 模式已启用，性能优化参数已应用
```
以及扫描日志中的 "已成功过，跳过" 信息。

### Q: 可以同时使用配置文件和Web界面管理视频源吗？
A: 不可以。现在只支持通过Web界面管理视频源，配置文件中的视频源设置会被忽略。

### Q: 如何备份我的视频源设置？
A: 视频源现在存储在SQLite数据库中（通常在 `data/bili_sync.db`），备份这个文件即可。

### Q: Web界面无法访问怎么办？
A: 检查 `bind_address` 配置，确保端口没有被占用，防火墙允许访问。

### Q: 如何确认视频没有重复下载？
A: 查看日志输出，已下载的视频会显示 "已成功过，跳过" 或者系统会报告 "找到 0 个未处理完成的视频"。

## 🔧 技术细节

### v2.7.3 新增配置表

```sql
-- 配置项存储表
CREATE TABLE config_items (
    key_name TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 配置变更历史表
CREATE TABLE config_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key_name TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT NOT NULL,
    changed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 配置热重载架构

```rust
// ArcSwap 模式实现
static CONFIG_BUNDLE: Lazy<ArcSwap<ConfigBundle>> = ...

pub fn with_config<F, R>(f: F) -> R 
where F: FnOnce(&ConfigBundle) -> R
```

### 文件名处理规则

1. **字符映射表**
   - `：` → `-` (全角冒号)
   - `「」` → `[]` (日文引号)
   - `（）` → `()` (全角括号)
   - `《》` → `_` (书名号)

2. **保留字符处理**
   - Windows: `< > : " / \ | ? *`
   - 控制字符: `\u{0000}-\u{001F}`

3. **安全保障**
   - 长度限制: 200 字符
   - UTF-8 边界检查
   - 空名称默认: "unnamed"

### API 端点更新

#### 配置管理 API (v2.7.3 新增)
- `GET /api/config/{key}` - 获取配置项
- `PUT /api/config/{key}` - 更新配置项
- `POST /api/config/batch` - 批量更新
- `POST /api/config/reload` - 触发热重载
- `GET /api/config/history` - 查看历史

#### 初始设置 API (v2.7.3 新增)
- `POST /api/initial-setup` - 保存初始设置
- `GET /api/initial-setup/status` - 检查设置状态

## 回滚方案

如果需要回滚到旧版本：

1. 恢复备份的配置文件：`cp config.toml.backup config.toml`
2. 使用旧版本的程序
3. 注意：数据库中的视频源数据需要手动转换回配置文件格式

---

如有问题，请查看日志文件或提交Issue。 