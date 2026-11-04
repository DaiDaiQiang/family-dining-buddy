/**
 * 图片压缩工具
 *
 * 手机原图动辄 5-10MB，上传前先等比缩到 maxSize 以内并转 JPEG，
 * 典型产物 100~300KB，正好匹配后端 2MB 的 base64 解码上限。
 */

/**
 * 压缩图片文件为 JPEG dataURL
 * @param {File|Blob} file 用户选择的图片文件
 * @param {object} opts maxSize 最长边像素（默认 1080）；quality JPEG 质量（默认 0.85）
 * @returns {Promise<string>} data:image/jpeg;base64,...
 */
export function compressToJpeg(file, { maxSize = 1080, quality = 0.85 } = {}) {
  return createBitmap(file).then(({ bitmap, width, height }) => {
    const scale = Math.min(1, maxSize / Math.max(width, height))
    const w = Math.max(1, Math.round(width * scale))
    const h = Math.max(1, Math.round(height * scale))

    const canvas = document.createElement('canvas')
    canvas.width = w
    canvas.height = h
    const ctx = canvas.getContext('2d')
    ctx.drawImage(bitmap, 0, 0, w, h)
    bitmap.close?.()

    return canvas.toDataURL('image/jpeg', quality)
  })
}

/** 优先用 createImageBitmap（自动应用 EXIF 方向），旧浏览器回退到 <img> */
function createBitmap(file) {
  if (typeof createImageBitmap === 'function') {
    return createImageBitmap(file, { imageOrientation: 'from-image' })
      .then((bmp) => ({ bitmap: bmp, width: bmp.width, height: bmp.height }))
      .catch(() => bitmapViaImg(file))
  }
  return bitmapViaImg(file)
}

function bitmapViaImg(file) {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file)
    const img = new Image()
    img.onload = () => {
      URL.revokeObjectURL(url)
      resolve({ bitmap: img, width: img.naturalWidth, height: img.naturalHeight })
    }
    img.onerror = () => {
      URL.revokeObjectURL(url)
      reject(new Error('图片读取失败'))
    }
    img.src = url
  })
}
