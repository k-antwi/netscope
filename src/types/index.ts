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

export interface InboundConnection {
  process: string
  pid: number
  local_ip: string
  local_port: number
  remote_ip: string
  remote_port: number
  state: 'LISTEN' | 'ESTABLISHED'
  is_encrypted: boolean
  is_localhost_only: boolean
  is_all_interfaces: boolean
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

export interface FileProcess {
  pid: number
  name: string
  access: string
}

export interface FileDetails {
  path: string
  size: number
  modified: number | null
  created: number | null
  is_dir: boolean
  permissions: string
  kind: string
  processes: FileProcess[]
}

export interface MalwareBazaarInfo {
  found: boolean
  signature: string | null
  file_type: string | null
  tags: string[]
  first_seen: string | null
}

export interface MalwareCheckResult {
  status: 'clean' | 'malicious' | 'suspicious' | 'unknown' | 'no_api_key' | 'error'
  sha256: string
  malicious: number
  suspicious: number
  harmless: number
  undetected: number
  total_engines: number
  permalink: string | null
  message: string | null
  malware_bazaar: MalwareBazaarInfo | null
}

export interface ScanSummary {
  matches: number
  scannedDirs: number
  elapsedMs: number | null
  isScanning: boolean
  truncated: boolean
}

export interface RemoteTrace {
  ip: string
  rdns: string
  org: string
  country: string
  city: string
}

export interface ServiceInvestigation {
  pid: number
  process_path: string
  local_port: number
  local_ip: string
  service_name: string
  exposure: string
  active_connections: number
  is_encrypted: boolean
  warnings: string[]
  active_remotes: RemoteTrace[]
}

export interface CveEntry {
  id: string
  description: string
  severity: string | null
  score: number | null
  published: string | null
  url: string
}

export interface CveCheckResult {
  query: string
  total_results: number
  cves: CveEntry[]
  message: string | null
}

export interface Issue {
  severity: 'critical' | 'high' | 'warning' | 'info'
  category: string
  title: string
  detail: string
  process: string
  pid: number
  port: number | null
  remote_ip: string | null
}

export interface BrowserHeader {
  name: string
  value: string
}

export interface BrowserRequest {
  id: string
  url: string
  method: string
  status: number
  statusText: string
  requestHeaders: BrowserHeader[]
  responseHeaders: BrowserHeader[]
  requestBody: string | null
  timingMs: number
  fromCache: boolean
  initiator: string
  tabUrl: string
  timestamp: number
  error: string | null
}

export const PORT_LABELS: Record<number, string> = {
  21: 'FTP',
  22: 'SSH',
  25: 'SMTP',
  53: 'DNS',
  80: 'HTTP',
  443: 'HTTPS',
  465: 'SMTPS',
  587: 'SMTP',
  993: 'IMAPS',
  3306: 'MySQL',
  5432: 'Postgres',
  5228: 'FCM',
  6379: 'Redis',
  8080: 'HTTP-ALT',
  8443: 'HTTPS-ALT',
  27017: 'MongoDB',
}
