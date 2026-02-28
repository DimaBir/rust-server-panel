export interface GameServer {
  id: string
  name: string
  online: boolean
}

export interface ServerInfo {
  hostname: string
  players: number
  maxPlayers: number
  fps: number
  entities: number
  uptime: number
  map: string
  online: boolean
  cpuPercent: number
  memTotal: number
  memUsed: number
  memPercent: number
  diskTotal: number
  diskUsed: number
  diskPercent: number
}

export interface SystemStats {
  cpuPercent: number
  memTotal: number
  memUsed: number
  memPercent: number
  diskTotal: number
  diskUsed: number
  diskPercent: number
  timestamp: string
}

export interface GameStats {
  online: boolean
  players: number
  maxPlayers: number
  fps: number
  entities: number
  uptime: number
  map: string
  hostname: string
  timestamp: string
}

export interface MonitorResponse<T> {
  current: T | null
  history: T[]
}

export interface Player {
  steamId: string
  displayName: string
  address: string
  ping: number
  connectedSeconds: number
  health: number
}

export interface FileEntry {
  name: string
  path: string
  isDir: boolean
  size: number
  modified: string
  isText: boolean
}

export interface Plugin {
  name: string
  filename: string
  size: number
  modified: string
  hasConfig: boolean
}

export interface UModPlugin {
  name: string
  title: string
  description: string
  author: string
  version: string
  downloads: number
  downloadUrl: string
}

export interface ScheduledJob {
  id: string
  name: string
  jobType: string
  schedule: string
  payload: string | null
  enabled: boolean
  lastRun: string | null
  nextRun: string | null
  createdAt: string
  serverId: string
}

export interface LoginRequest {
  username: string
  password: string
}

export interface LoginResponse {
  token: string
  username: string
  expiresAt: string
}

export interface ApiError {
  error: string
}
