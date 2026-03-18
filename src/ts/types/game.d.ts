// TypeScript类型定义文件
// 预留扩展接口

export interface GameConfig {
  worldWidth: number;
  worldHeight: number;
  shipMaxSpeed: number;
  bulletSpeed: number;
}

export interface Ship {
  id: number;
  faction: 'Red' | 'Green' | 'Blue';
  position: { x: number; y: number };
  health: number;
}

export interface GameState {
  ships: Ship[];
  bullets: Bullet[];
  time: number;
}

export interface Bullet {
  id: number;
  shooterId: number;
  position: { x: number; y: number };
  velocity: { x: number; y: number };
}
