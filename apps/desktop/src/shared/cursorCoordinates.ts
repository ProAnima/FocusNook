export interface PhysicalPoint {
  x: number;
  y: number;
}

function usablePixelRatio(pixelRatio: number) {
  return Number.isFinite(pixelRatio) && pixelRatio > 0 ? pixelRatio : 1;
}

export function physicalCursorToClient(
  cursor: PhysicalPoint,
  windowPosition: PhysicalPoint,
  pixelRatio: number,
) {
  const ratio = usablePixelRatio(pixelRatio);
  return {
    x: (cursor.x - windowPosition.x) / ratio,
    y: (cursor.y - windowPosition.y) / ratio,
  };
}
