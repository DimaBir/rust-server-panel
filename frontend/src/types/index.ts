export interface ServerInfo {
  hostname: string
  players: number
  maxPlayers: number
  fps: number
  entities: number
  uptime: number
  map: string
  online: boolean
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
  timestamp: string
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
  fileName: string
  size: number
  modifiedAt: string
  hasConfig: boolean
  loaded: boolean
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
  schedule: string
  action: string
  params: Record<string, string>
  enabled: boolean
}

export interface LoginRequest {
  username: string
  password: string
}

export interface LoginResponse {
  token: string
}

export interface ApiError {
  error: string
  message: string
}
