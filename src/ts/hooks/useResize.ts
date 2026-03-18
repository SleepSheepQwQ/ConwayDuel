// Resize Hook
// 预留扩展接口

export function useResize(
  callback: (width: number, height: number) => void
): () => void {
  const observer = new ResizeObserver((entries) => {
    for (const entry of entries) {
      const { width, height } = entry.contentRect;
      callback(width, height);
    }
  });

  return () => observer.disconnect();
}

export function setupResizeListener(
  element: HTMLElement,
  callback: (width: number, height: number) => void
): () => void {
  const observer = new ResizeObserver((entries) => {
    for (const entry of entries) {
      const { width, height } = entry.contentRect;
      callback(width, height);
    }
  });

  observer.observe(element);

  return () => observer.disconnect();
}
