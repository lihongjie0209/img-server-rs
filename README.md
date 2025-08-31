# Image Compression Server

一个高性能的 Rust 图片压缩服务，支持多种压缩算法和流式处理。

## 🌐 在线演示

访问在线演示地址体验服务功能：**https://img.lihongjie.cn/**

- 支持直接上传图片进行压缩测试
- 实时查看压缩效果和统计信息
- 支持多种压缩算法和质量参数调节

## 功能特性

- **多种压缩算法支持**:
  - Mozilla JPEG (mozjpeg) - 高质量压缩
  - JPEG Encoder (jpeg-encoder) - 快速压缩
  - PNG 量化压缩 (png-quantized) - PNG 颜色量化

- **流式处理**: 支持大文件的内存优化处理
- **详细统计信息**: 通过响应头返回压缩统计数据
- **灵活的参数配置**: 支持查询参数和表单数据双重配置
- **高性能**: 基于 Actix-web 框架，支持异步处理

## API 接口

### 压缩接口

**POST** `/compress`

**Content-Type**: `multipart/form-data`

#### 请求参数

| 参数 | 类型 | 必需 | 描述 |
|------|------|------|------|
| `file` | File | 是 | 要压缩的图片文件 |
| `quality` | Integer | 否 | 压缩质量 (1-100)，默认: 80 |
| `algorithm` | String | 否 | 压缩算法，默认: mozjpeg |

**支持的算法**:
- `mozjpeg` - Mozilla JPEG 编码器（高质量）
- `jpeg-encoder` / `fast-jpeg` - 快速 JPEG 编码器
- `png-quantized` / `png` - PNG 颜色量化压缩

#### 查询参数（可选）

也可以通过 URL 查询参数传递配置：
```
POST /compress?quality=85&algorithm=mozjpeg
```

#### 响应

成功时返回压缩后的图片文件，并在响应头中包含统计信息：

- `X-Original-Size`: 原始文件大小（字节）
- `X-Compressed-Size`: 压缩后文件大小（字节）
- `X-Compression-Ratio`: 压缩比例（百分比）
- `X-Processing-Time-Ms`: 处理时间（毫秒）
- `X-Image-Width`: 图片宽度
- `X-Image-Height`: 图片高度
- `X-Algorithm-Used`: 使用的压缩算法

### 其他接口

- **GET** `/` - 服务信息
- **GET** `/health` - 健康检查
- **GET** `/info` - 详细的 API 信息

## 快速开始

### 编译运行

```bash
# 克隆项目
git clone <repository-url>
cd img-server-rs

# 编译
cargo build --release

# 运行
cargo run --release
```

服务将在 `http://0.0.0.0:8080` 启动。

### 使用示例

#### curl 命令

```bash
# 基本压缩（使用默认设置）
curl -X POST http://localhost:8080/compress \
  -F "file=@example.jpg" \
  -o compressed.jpg

# 指定压缩质量和算法
curl -X POST http://localhost:8080/compress \
  -F "file=@example.png" \
  -F "quality=90" \
  -F "algorithm=png-quantized" \
  -o compressed.png

# 使用查询参数
curl -X POST "http://localhost:8080/compress?quality=70&algorithm=jpeg-encoder" \
  -F "file=@example.jpg" \
  -o compressed.jpg

# 查看统计信息
curl -X POST http://localhost:8080/compress \
  -F "file=@example.jpg" \
  -D headers.txt \
  -o compressed.jpg
cat headers.txt
```

#### Python 示例

```python
import requests

# 上传并压缩图片
with open('example.jpg', 'rb') as f:
    files = {'file': f}
    data = {'quality': 85, 'algorithm': 'mozjpeg'}
    
    response = requests.post(
        'http://localhost:8080/compress',
        files=files,
        data=data
    )
    
    if response.status_code == 200:
        # 保存压缩后的文件
        with open('compressed.jpg', 'wb') as out:
            out.write(response.content)
        
        # 打印统计信息
        print(f"原始大小: {response.headers['X-Original-Size']} 字节")
        print(f"压缩后大小: {response.headers['X-Compressed-Size']} 字节")
        print(f"压缩比: {response.headers['X-Compression-Ratio']}")
        print(f"处理时间: {response.headers['X-Processing-Time-Ms']} 毫秒")
    else:
        print(f"压缩失败: {response.text}")
```

#### JavaScript 示例

```javascript
const formData = new FormData();
formData.append('file', fileInput.files[0]);
formData.append('quality', '80');
formData.append('algorithm', 'mozjpeg');

fetch('http://localhost:8080/compress', {
    method: 'POST',
    body: formData
})
.then(response => {
    if (response.ok) {
        // 获取统计信息
        console.log('原始大小:', response.headers.get('X-Original-Size'));
        console.log('压缩比:', response.headers.get('X-Compression-Ratio'));
        
        return response.blob();
    }
    throw new Error('压缩失败');
})
.then(blob => {
    // 处理压缩后的文件
    const url = URL.createObjectURL(blob);
    // 下载或显示文件
})
.catch(error => console.error('错误:', error));
```

## 性能特性

- **内存优化**: 
  - PNG 压缩支持零拷贝优化（当内存布局兼容时）
  - JPEG 压缩使用逐行处理，减少内存占用
  - 大文件流式处理，避免全部加载到内存

- **并发处理**: 基于 Actix-web 的异步架构，支持高并发请求

- **文件大小限制**: 默认最大支持 100MB 文件上传

## 配置

### 环境变量

- `RUST_LOG`: 日志级别（默认: info）

### 编译选项

在 `Cargo.toml` 中可以启用额外功能：

```toml
[features]
default = []
webp = ["image/webp"]  # 启用 WebP 支持（实验性）
```

## 性能基准

根据参考实现，该服务具有以下性能特征：

- **PNG 压缩**: 支持零拷贝优化，大图片可节省约 50% 内存使用
- **JPEG 压缩**: 逐行处理，内存使用稳定
- **处理速度**: 根据图片大小和复杂度，通常在几十到几百毫秒之间

## 故障排除

### 常见错误

1. **文件过大**: 默认限制 100MB，可通过修改 `MAX_PAYLOAD_SIZE` 调整
2. **不支持的格式**: 目前支持 PNG、JPEG 格式
3. **内存不足**: 对于极大图片，考虑增加系统内存或降低并发数

### 日志

启用详细日志：
```bash
RUST_LOG=debug cargo run
```

## 许可证

[添加您的许可证信息]

## 贡献

欢迎提交 Issue 和 Pull Request！
