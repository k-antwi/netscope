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
