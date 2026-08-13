import type { Issue } from '../types'

export interface Remediation {
  impact: string[]
  steps: string[]
  configHint?: string  // optional inline config example
}

const DATA: Record<string, Remediation> = {
  'exposed-database': {
    impact: [
      'Any network client — including automated scanners — can attempt to authenticate directly to the database.',
      'A weak or default password grants immediate read/write access to all stored data.',
      'Attackers can extract, modify, or permanently delete every record in the database.',
      'Ransomware frequently targets exposed database ports to encrypt data and demand payment.',
      'Regulatory exposure: if personal or payment data is stored, this may violate GDPR, HIPAA, or PCI-DSS.',
    ],
    steps: [
      'Open the database configuration file and set the bind address to 127.0.0.1 so it only accepts local connections.',
      'Restart the database service to apply the change.',
      'Add a firewall rule to block inbound traffic on this port from any external interface.',
      'If remote access is genuinely required, use an SSH tunnel (ssh -L local_port:127.0.0.1:db_port user@host) rather than opening the port to the internet.',
      'Audit all database user accounts: remove anonymous users and any accounts with empty or default passwords.',
      'Enable database access logging and review logs for any past unauthorised connection attempts.',
    ],
    configHint:
      'MySQL: bind-address = 127.0.0.1  (in /etc/mysql/my.cnf)\n' +
      'PostgreSQL: listen_addresses = \'localhost\'  (in postgresql.conf)\n' +
      'Redis: bind 127.0.0.1  (in /etc/redis/redis.conf)\n' +
      'MongoDB: net.bindIp: 127.0.0.1  (in /etc/mongod.conf)',
  },

  'plaintext-service-telnet': {
    impact: [
      'Every keystroke — including passwords — is transmitted in cleartext and is visible to any observer on the same network.',
      'Trivial to capture full session contents with a packet sniffer (Wireshark, tcpdump).',
      'Credentials obtained can be used to escalate access to other systems.',
      'Modern systems have no legitimate use case for Telnet; its presence is a serious misconfiguration.',
    ],
    steps: [
      'Stop the Telnet service immediately: sudo launchctl disable system/telnet (macOS) or sudo systemctl stop telnet (Linux).',
      'Remove the Telnet package if installed: sudo apt remove telnetd (Debian/Ubuntu) or brew uninstall inetutils.',
      'Ensure OpenSSH is installed and running as the replacement: ssh -V should return a version string.',
      'Configure SSH key-based authentication and disable password login by setting PasswordAuthentication no in /etc/ssh/sshd_config.',
      'Verify Telnet is not re-enabled on boot by checking startup scripts and launchd plists.',
    ],
  },

  'plaintext-service-ftp': {
    impact: [
      'Login credentials (username and password) are transmitted in cleartext — any network observer can capture them.',
      'All transferred file contents are visible in transit, including sensitive documents.',
      'FTP is vulnerable to bounce attacks, allowing attackers to relay connections through your server.',
      'Captured FTP credentials are frequently reused to access other services.',
    ],
    steps: [
      'Migrate immediately to SFTP (SSH File Transfer Protocol) — no extra software needed if SSH is already running on the server.',
      'Update all FTP client tools and scripts to use sftp:// URLs in place of ftp://',
      'If SFTP is not viable, configure FTPS (FTP over TLS) in your FTP server settings (Explicit TLS on port 21 or Implicit TLS on port 990).',
      'Stop and disable the plain FTP service: sudo launchctl disable system/ftp (macOS) or sudo systemctl disable vsftpd (Linux).',
    ],
  },

  'unencrypted-exposure-http': {
    impact: [
      'All HTTP traffic — including session cookies, form submissions, and authentication tokens — is readable in transit.',
      'Man-in-the-middle attackers can inject malicious scripts into HTTP responses seen by your users.',
      'Browsers and search engines actively warn users about and penalise unencrypted HTTP pages.',
      'Session cookies without the Secure flag sent over HTTP can be stolen and replayed.',
    ],
    steps: [
      'Obtain a free TLS certificate from Let\'s Encrypt using Certbot: sudo certbot --nginx or sudo certbot --apache.',
      'Configure your web server to listen on port 443 (HTTPS) and redirect all port 80 traffic to HTTPS.',
      'Add an HSTS response header to prevent future downgrade attacks: Strict-Transport-Security: max-age=31536000; includeSubDomains.',
      'Mark session cookies with the Secure and HttpOnly flags so they are never sent over plain HTTP.',
      'Verify your TLS setup using the SSL Labs server test (search "SSL Server Test" online).',
    ],
    configHint:
      'nginx redirect example:\n' +
      'server {\n' +
      '  listen 80;\n' +
      '  return 301 https://$host$request_uri;\n' +
      '}',
  },

  'unencrypted-exposure-other': {
    impact: [
      'All traffic between clients and this service can be read or modified by anyone on the network path.',
      'Authentication credentials sent to this service may be captured and reused.',
      'Data integrity cannot be verified without encryption — responses could be tampered with in transit.',
    ],
    steps: [
      'Check the service documentation for a built-in TLS or SSL option and enable it in the service configuration.',
      'If the service does not support TLS natively, place a TLS-terminating reverse proxy in front of it (nginx, Caddy, or stunnel).',
      'If external access is not required, change the bind address to 127.0.0.1 so the service is only reachable locally.',
      'If limited external access is needed, restrict it to specific trusted IP ranges using firewall rules rather than leaving it open to all.',
    ],
  },

  'plaintext-outbound': {
    impact: [
      'HTTP traffic can be read by any network observer between your machine and the destination server.',
      'Session cookies or API tokens sent over HTTP can be stolen and used to impersonate you.',
      'Content received over HTTP can be injected or modified in transit without detection.',
      'Privacy: browsing history and request parameters are visible to ISPs and network operators.',
    ],
    steps: [
      'Locate the application\'s settings or config file and replace any http:// endpoint URLs with https://.',
      'If the application is third-party software, update to the latest version — modern versions typically default to HTTPS.',
      'Check the application\'s proxy or network settings for an option to enforce HTTPS-only connections.',
      'If you cannot control the application, consider routing its traffic through a local HTTPS-enforcing proxy.',
      'If the destination server does not offer HTTPS, report this to the service operator as a security issue.',
    ],
  },

  'unusual-port': {
    impact: [
      'Non-standard port usage may indicate malware or command-and-control (C2) communication.',
      'Could represent data exfiltration bypassing monitoring tools that focus on standard ports.',
      'Even if the application is legitimate, unusual port activity should be verified to establish a baseline.',
    ],
    steps: [
      'Identify the full executable path: run ps -p <pid> -o args= in Terminal to see exactly which binary made the connection.',
      'Cross-reference the process with what you installed — search the executable name online if unfamiliar.',
      'Check the application\'s documentation to confirm whether this port is expected behaviour (e.g. a custom API port).',
      'If the connection is unexpected, quit the application and run a malware scan.',
      'Consider adding a firewall outbound rule to block this port until the source is confirmed as legitimate.',
    ],
  },
}

export function getRemediation(issue: Issue): Remediation {
  if (issue.category === 'plaintext-service') {
    return issue.port === 23 ? DATA['plaintext-service-telnet'] : DATA['plaintext-service-ftp']
  }
  if (issue.category === 'unencrypted-exposure') {
    return issue.port === 80 ? DATA['unencrypted-exposure-http'] : DATA['unencrypted-exposure-other']
  }
  return DATA[issue.category] ?? {
    impact: ['The nature of the impact depends on the specific service and context.'],
    steps: ['Investigate the process and port using Activity Monitor or lsof in Terminal.'],
  }
}
