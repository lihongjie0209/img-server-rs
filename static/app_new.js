class ModernImageCompressor {
    constructor() {
        this.init();
        this.serverUrl = '';
        this.selectedFile = null;
        this.isCompressed = false;
    }

    init() {
        this.bindEvents();
        this.updateQualityDisplay();
    }

    bindEvents() {
        // 文件上传事件
        const fileInput = document.getElementById('fileInput');
        const uploadZone = document.getElementById('uploadZone');

        fileInput.addEventListener('change', (e) => this.handleFileSelect(e));

        // 上传区域点击事件
        uploadZone.addEventListener('click', () => {
            fileInput.click();
        });

        // 拖拽事件
        uploadZone.addEventListener('dragover', (e) => this.handleDragOver(e));
        uploadZone.addEventListener('dragleave', (e) => this.handleDragLeave(e));
        uploadZone.addEventListener('drop', (e) => this.handleDrop(e));

        // 质量滑块事件
        const qualitySlider = document.getElementById('quality');
        qualitySlider.addEventListener('input', () => this.updateQualityDisplay());

        // 压缩按钮事件
        const compressBtn = document.getElementById('compressBtn');
        compressBtn.addEventListener('click', () => this.compressImage());

        // 重新压缩按钮事件
        const recompressBtn = document.getElementById('recompressBtn');
        recompressBtn.addEventListener('click', () => this.recompressImage());

        // 算法选择事件
        const algorithmSelect = document.getElementById('algorithm');
        algorithmSelect.addEventListener('change', () => this.updateAlgorithmInfo());
    }

    handleDragOver(e) {
        e.preventDefault();
        e.stopPropagation();
        document.getElementById('uploadZone').classList.add('dragover');
    }

    handleDragLeave(e) {
        e.preventDefault();
        e.stopPropagation();
        document.getElementById('uploadZone').classList.remove('dragover');
    }

    handleDrop(e) {
        e.preventDefault();
        e.stopPropagation();
        document.getElementById('uploadZone').classList.remove('dragover');

        const files = e.dataTransfer.files;
        if (files.length > 0) {
            this.processFile(files[0]);
        }
    }

    handleFileSelect(e) {
        const file = e.target.files[0];
        if (file) {
            this.processFile(file);
        }
    }

    processFile(file) {
        // 验证文件类型
        if (!file.type.startsWith('image/')) {
            this.showError('请选择有效的图片文件');
            return;
        }

        // 验证文件大小 (100MB)
        if (file.size > 100 * 1024 * 1024) {
            this.showError('文件大小不能超过 100MB');
            return;
        }

        this.selectedFile = file;
        this.setupOriginalImage(file);
        this.enableCompressButton();
        this.hideError();
    }

    setupOriginalImage(file) {
        const reader = new FileReader();
        reader.onload = (e) => {
            const originalImage = document.getElementById('originalImage');
            originalImage.src = e.target.result;

            // 更新原始图片信息
            document.getElementById('originalSize').textContent = this.formatFileSize(file.size);

            // 获取图片尺寸
            originalImage.onload = () => {
                document.getElementById('originalDimensions').textContent =
                    `${originalImage.naturalWidth} × ${originalImage.naturalHeight}`;
            };
        };
        reader.readAsDataURL(file);
    }

    enableCompressButton() {
        document.getElementById('compressBtn').disabled = false;
    }

    updateQualityDisplay() {
        const quality = document.getElementById('quality').value;
        const qualityValue = document.getElementById('qualityValue');

        let qualityText = quality;
        if (quality <= 30) qualityText += ' (小文件)';
        else if (quality <= 60) qualityText += ' (平衡)';
        else if (quality <= 85) qualityText += ' (高质量)';
        else qualityText += ' (最高质量)';

        qualityValue.textContent = qualityText;

        // 如果已经压缩过，自动启用重新压缩
        if (this.isCompressed) {
            this.showRecompressButton();
        }
    }

    updateAlgorithmInfo() {
        // 如果已经压缩过，自动启用重新压缩
        if (this.isCompressed) {
            this.showRecompressButton();
        }
    }

    async compressImage() {
        if (!this.selectedFile) {
            this.showError('请先选择图片文件');
            return;
        }

        this.showLoading();
        this.hideError();

        try {
            const result = await this.performCompression();
            this.displayComparisonResult(result);
            this.showSuccess('图片压缩完成！');
            this.isCompressed = true;
        } catch (error) {
            console.error('压缩失败:', error);
            this.showError(`压缩失败: ${error.message}`);
            this.showUploadZone();
        }
    }

    async recompressImage() {
        this.hideResults();
        await this.compressImage();
    }

    async performCompression() {
        const formData = new FormData();
        formData.append('file', this.selectedFile);
        formData.append('quality', document.getElementById('quality').value);
        formData.append('algorithm', document.getElementById('algorithm').value);

        // 获取目标格式
        const algorithm = document.getElementById('algorithm').value;
        let format = 'jpeg';
        if (algorithm === 'png') {
            format = 'png';
        }
        formData.append('format', format);

        const startTime = Date.now();

        const response = await fetch(`${this.serverUrl}/compress`, {
            method: 'POST',
            body: formData
        });

        const processingTime = Date.now() - startTime;

        if (!response.ok) {
            let errorMessage = `服务器错误: ${response.status}`;
            try {
                const errorData = await response.json();
                errorMessage = errorData.error || errorMessage;
            } catch (e) {
                // 如果响应不是JSON，使用默认错误消息
            }
            throw new Error(errorMessage);
        }

        const compressedBlob = await response.blob();

        // 获取响应头中的统计信息
        const stats = this.extractStatsFromHeaders(response.headers);
        stats.clientProcessingTime = processingTime;
        stats.originalSize = this.selectedFile.size;
        stats.compressedSize = compressedBlob.size;

        return { blob: compressedBlob, stats };
    }

    extractStatsFromHeaders(headers) {
        return {
            originalSize: parseInt(headers.get('X-Original-Size') || this.selectedFile?.size || '0'),
            compressedSize: parseInt(headers.get('X-Compressed-Size') || '0'),
            compressionRatio: parseFloat(headers.get('X-Compression-Ratio') || '0'),
            processingTime: parseInt(headers.get('X-Processing-Time-Ms') || '0'),
            imageWidth: parseInt(headers.get('X-Image-Width') || '0'),
            imageHeight: parseInt(headers.get('X-Image-Height') || '0'),
            algorithmUsed: headers.get('X-Algorithm-Used') || document.getElementById('algorithm').value
        };
    }

    displayComparisonResult(result) {
        const { blob, stats } = result;

        // 显示压缩后的图片
        const compressedUrl = URL.createObjectURL(blob);
        const compressedImg = document.getElementById('compressedImage');
        compressedImg.src = compressedUrl;

        // 计算实际的压缩数据
        const actualOriginalSize = stats.originalSize || this.selectedFile?.size || 0;
        const actualCompressedSize = stats.compressedSize || blob.size;
        const actualCompressionRatio = actualOriginalSize > 0 ?
            ((actualOriginalSize - actualCompressedSize) / actualOriginalSize * 100) : 0;

        // 更新压缩后图片信息
        document.getElementById('compressedSize').textContent = this.formatFileSize(actualCompressedSize);
        document.getElementById('compressedDimensions').textContent =
            stats.imageWidth && stats.imageHeight ?
            `${stats.imageWidth} × ${stats.imageHeight}` : '计算中...';

        // 显示压缩率
        const savedSize = actualOriginalSize - actualCompressedSize;
        const compressionText = `压缩率: ${actualCompressionRatio.toFixed(1)}% (节省 ${this.formatFileSize(savedSize)})`;
        document.getElementById('compressionText').textContent = compressionText;

        // 设置下载链接
        const downloadBtn = document.getElementById('downloadBtn');
        downloadBtn.href = compressedUrl;
        downloadBtn.download = this.generateDownloadFilename(stats.algorithmUsed);

        // 显示结果
        this.showComparisonResult();
    }

    generateDownloadFilename(algorithm) {
        const originalName = this.selectedFile.name;
        const nameWithoutExt = originalName.substring(0, originalName.lastIndexOf('.'));
        const ext = algorithm === 'png' ? 'png' : 'jpg';
        return `${nameWithoutExt}_compressed_${algorithm}.${ext}`;
    }

    formatFileSize(bytes) {
        if (bytes === 0) return '0 Bytes';

        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));

        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    // UI状态管理
    showLoading() {
        document.getElementById('uploadZone').style.display = 'none';
        document.getElementById('comparisonZone').classList.remove('active');
        document.getElementById('loading').classList.add('active');
        this.hideButtons();
    }

    hideLoading() {
        document.getElementById('loading').classList.remove('active');
    }

    showUploadZone() {
        document.getElementById('uploadZone').style.display = 'flex';
        document.getElementById('comparisonZone').classList.remove('active');
        document.getElementById('compressionResult').classList.remove('active');
        this.hideLoading();
        this.hideButtons();
        document.getElementById('compressBtn').style.display = 'inline-flex';
    }

    showComparisonResult() {
        this.hideLoading();
        document.getElementById('uploadZone').style.display = 'none';
        document.getElementById('comparisonZone').classList.add('active');
        document.getElementById('compressionResult').classList.add('active');

        // 显示操作按钮
        this.hideButtons();
        document.getElementById('recompressBtn').style.display = 'inline-flex';
        document.getElementById('downloadBtn').style.display = 'inline-flex';
        document.getElementById('nextBtn').style.display = 'inline-flex';
    }

    showRecompressButton() {
        if (this.isCompressed) {
            document.getElementById('recompressBtn').style.display = 'inline-flex';
        }
    }

    hideResults() {
        document.getElementById('comparisonZone').classList.remove('active');
        document.getElementById('compressionResult').classList.remove('active');
    }

    hideButtons() {
        document.getElementById('compressBtn').style.display = 'none';
        document.getElementById('recompressBtn').style.display = 'none';
        document.getElementById('downloadBtn').style.display = 'none';
        document.getElementById('nextBtn').style.display = 'none';
    }

    showError(message) {
        const errorEl = document.getElementById('errorMsg');
        errorEl.textContent = message;
        errorEl.classList.add('active');

        // 5秒后自动隐藏
        setTimeout(() => {
            errorEl.classList.remove('active');
        }, 5000);
    }

    hideError() {
        document.getElementById('errorMsg').classList.remove('active');
    }

    showSuccess(message) {
        const successEl = document.getElementById('successMsg');
        successEl.textContent = message;
        successEl.classList.add('active');

        // 3秒后自动隐藏
        setTimeout(() => {
            successEl.classList.remove('active');
        }, 3000);
    }
}

// 页面加载完成后初始化
document.addEventListener('DOMContentLoaded', () => {
    new ModernImageCompressor();
});

// 添加键盘快捷键支持
document.addEventListener('keydown', (e) => {
    // Ctrl/Cmd + O 打开文件
    if ((e.ctrlKey || e.metaKey) && e.key === 'o') {
        e.preventDefault();
        document.getElementById('fileInput').click();
    }

    // Enter 键压缩图片
    if (e.key === 'Enter' && !document.getElementById('compressBtn').disabled) {
        const compressBtn = document.getElementById('compressBtn');
        if (compressBtn.style.display !== 'none') {
            compressBtn.click();
        }
    }
});

// 清理对象URL，防止内存泄漏
window.addEventListener('beforeunload', () => {
    const images = document.querySelectorAll('img[src^="blob:"]');
    images.forEach(img => {
        URL.revokeObjectURL(img.src);
    });
});
