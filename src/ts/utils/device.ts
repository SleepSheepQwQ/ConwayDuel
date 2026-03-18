// 设备检测工具
// 预留扩展接口

export function isMobile(): boolean {
  return /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
}

export function getDevicePixelRatio(): number {
  return window.devicePixelRatio || 1;
}

export function isWebGL2Supported(): boolean {
  try {
    const canvas = document.createElement('canvas');
    return !!(canvas.getContext('webgl2'));
  } catch {
    return false;
  }
}

export function getScreenOrientation(): 'portrait' | 'landscape' {
  return window.innerWidth > window.innerHeight ? 'landscape' : 'portrait';
}
