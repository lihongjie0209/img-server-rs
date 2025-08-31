# 图像压缩服务部署指南

本文档说明如何使用迁移后的脚本将Rust图像压缩服务部署到Docker Hub和GCP Cloud Run。

## 🚀 部署架构

```
GitHub Repository
    ↓ (Push Tag v*)
GitHub Actions
    ↓ (Build & Test)
Docker Hub
    ↓ (Deploy Image)
GCP Cloud Run
    ↓ (Serve Users)
Public API
```

## 📋 前置条件

### 1. 必要工具
- **Google Cloud SDK**: [下载安装](https://cloud.google.com/sdk/docs/install)
- **GitHub CLI**: [下载安装](https://cli.github.com/)
- **Python 3.8+**: 用于运行配置脚本
- **Git**: 版本控制

### 2. 账户准备
- **GCP账户**: 需要计费账户权限
- **GitHub账户**: 仓库管理权限
- **Docker Hub账户**: 镜像推送权限

### 3. 权限设置
- GCP项目创建/管理权限
- GitHub仓库Secrets和Variables配置权限
- Docker Hub镜像推送权限

## ⚙️ 配置步骤

### 第一步：安装Python依赖

```bash
# 安装GCP配置工具依赖
pip install -r scripts/requirements-gcp.txt

# 安装发布工具依赖  
pip install -r scripts/requirements-release.txt
```

### 第二步：配置GCP项目

```bash
# 运行GCP初始化脚本
python scripts/gcp_init.py

# 按提示完成以下步骤：
# 1. 登录GCP账户
# 2. 选择或创建项目
# 3. 启用必要的API服务
# 4. 创建服务账户和权限
# 5. 自动配置GitHub Secrets和Variables
```

### 第三步：配置Docker Hub

在GitHub仓库的 **Settings > Secrets and variables > Actions** 中手动添加：

| Secret名称 | 值 | 说明 |
|-----------|-----|------|
| `DOCKERHUB_USERNAME` | 你的Docker Hub用户名 | 用于登录Docker Hub |
| `DOCKERHUB_TOKEN` | Docker Hub访问令牌 | [创建令牌](https://hub.docker.com/settings/security) |

### 第四步：更新GitHub Actions配置

在 `.github/workflows/docker-build.yml` 中更新以下配置：

```yaml
env:
  DOCKERHUB_NAMESPACE: your-dockerhub-username  # 改为你的用户名
  IMAGE_NAME: img-server-rs
```

## 🔧 配置说明

### GitHub Secrets（自动配置）
| Secret | 用途 |
|--------|------|
| `GCP_SA_KEY` | GCP服务账户密钥（JSON格式） |
| `DOCKERHUB_USERNAME` | Docker Hub用户名（需手动配置） |
| `DOCKERHUB_TOKEN` | Docker Hub访问令牌（需手动配置） |

### GitHub Variables（自动配置）
| Variable | 默认值 | 说明 |
|----------|--------|------|
| `CLOUD_RUN_SERVICE_NAME` | `img-server-rs` | Cloud Run服务名称 |
| `GCP_REGION` | `asia-east2` | 部署区域（香港） |
| `CLOUD_RUN_MEMORY` | `1Gi` | 容器内存限制 |
| `CLOUD_RUN_CPU` | `1` | CPU核心数 |
| `CLOUD_RUN_CONCURRENCY` | `80` | 单实例并发数 |
| `CLOUD_RUN_MAX_INSTANCES` | `10` | 最大实例数 |
| `CLOUD_RUN_MIN_INSTANCES` | `0` | 最小实例数 |
| `RUST_LOG` | `info` | 日志级别 |

## 🚀 部署流程

### 1. 自动化部署（推荐）

创建并推送版本标签即可触发自动部署：

```bash
# 使用自动化发布脚本
python scripts/release.py --bump-type=patch

# 或手动创建标签
git tag v1.0.0
git push origin v1.0.0
```

### 2. 手动部署

也可以通过GitHub仓库的Actions页面手动触发：

1. 进入 **Actions** 页面
2. 选择 **Docker Build and Push** 工作流
3. 点击 **Run workflow**
4. 设置参数并运行

## 📊 部署验证

### 1. Docker Hub验证
访问 [Docker Hub](https://hub.docker.com/r/your-username/img-server-rs) 查看镜像是否成功推送。

### 2. Cloud Run验证
```bash
# 查看服务状态
gcloud run services list --region=asia-east2

# 获取服务URL
gcloud run services describe img-server-rs \
  --region=asia-east2 \
  --format='value(status.url)'
```

### 3. 功能测试
```bash
# 健康检查
curl https://your-service-url.run.app/

# 图像压缩测试
curl -X POST https://your-service-url.run.app/compress \
  -F "image=@test.jpg" \
  -F "quality=80" \
  -F "algorithm=mozjpeg"
```

## 🔧 故障排除

### 常见问题

1. **GCP认证失败**
   ```bash
   gcloud auth login
   gcloud config set project YOUR_PROJECT_ID
   ```

2. **GitHub CLI未认证**
   ```bash
   gh auth login
   ```

3. **Docker Hub推送失败**
   - 检查用户名和令牌是否正确
   - 确认Docker Hub仓库存在且有推送权限

4. **Cloud Run部署失败**
   - 检查GCP服务账户权限
   - 确认API服务已启用
   - 验证内存和CPU配置是否合理

### 日志查看

```bash
# GitHub Actions日志
# 在仓库的Actions页面查看详细日志

# Cloud Run日志
gcloud logging read "resource.type=cloud_run_revision" \
  --limit=50 \
  --format='table(timestamp,severity,textPayload)'

# Docker容器本地测试
docker run -p 3030:3030 your-username/img-server-rs:latest
```

## 📈 性能优化

### Cloud Run配置建议

- **内存**: 根据图像处理需求调整（1Gi-4Gi）
- **CPU**: 建议至少1核，处理量大时可增加到2-4核
- **并发**: 图像处理为CPU密集型，建议20-50
- **实例数**: 最小0实例节省成本，最大根据预期负载设置

### Docker镜像优化

- 使用多阶段构建减小镜像体积
- 优化Rust编译配置
- 合理配置基础镜像

## 🔒 安全最佳实践

1. **服务账户权限最小化**: 只授予必要的Cloud Run权限
2. **密钥轮换**: 定期更新服务账户密钥和Docker Hub令牌
3. **网络安全**: 配置合适的Cloud Run流量控制
4. **日志监控**: 启用Cloud Run监控和日志记录

## 📞 支持

如遇到问题，请检查：
1. 本文档的故障排除部分
2. GitHub Actions的执行日志
3. GCP Cloud Run的服务日志
4. 脚本执行时的错误输出

更多技术支持请参考：
- [GCP Cloud Run文档](https://cloud.google.com/run/docs)
- [GitHub Actions文档](https://docs.github.com/en/actions)
- [Docker Hub文档](https://docs.docker.com/docker-hub/)

---

*该部署指南基于Rust图像压缩服务项目，包含从GitHub Actions到Docker Hub再到GCP Cloud Run的完整自动化部署流程。*
