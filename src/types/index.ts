export interface Connection {
  process: string
  pid: number
  local_addr: string
  remote_ip: string
  remote_port: number
  state: string
  is_https: boolean
  is_local: boolean
}

export interface IpInvestigation {
  ip: string
  rdns: string
  asn_org: string
  country: string
  city: string
  tls_subject: string
  tls_issuer: string
  whois_netname: string
  whois_org: string
  suspicious: boolean
  suspicious_reasons: string[]
}

export interface FileMatch {
  name: string
  path: string
  parent: string
  size: number
  modified: number | null
  is_dir: boolean
  exact: boolean
}

export interface FileScanResult {
  query: string
  matches: FileMatch[]
  scanned_dirs: number
  roots: string[]
  elapsed_ms: number
  truncated: boolean
  cancelled: boolean
  denied: number
}

export interface ScanProgress {
  scanned_dirs: number
  found: number
  current: string
}

export interface ScanSummary {
  matches: number
  scannedDirs: number
  elapsedMs: number | null
  isScanning: boolean
  truncated: boolean
}

export const PORT_LABELS: Record<number, string> = {
  21: 'FTP',
  22: 'SSH',
  25: 'SMTP',
  80: 'HTTP',
  443: 'HTTPS',
  587: 'SMTP',
  993: 'IMAPS',
  5228: 'FCM',
  8080: 'HTTP-ALT',
  8443: 'HTTPS-ALT',
}
