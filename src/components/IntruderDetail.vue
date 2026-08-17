<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { IntruderFinding } from '../types'

const props = defineProps<{ finding: IntruderFinding }>()
defineEmits<{ close: [] }>()

const killing  = ref(false)
const killed   = ref(false)
const killError = ref('')

// Reset kill state whenever the selected finding changes
watch(() => props.finding.id, () => {
  killing.value  = false
  killed.value   = false
  killError.value = ''
})

// ── Encryption assessment ────────────────────────────────────────────────────

const ENCRYPTED_PORTS = new Set([443, 8443, 465, 587, 993, 995, 636, 989, 990, 5061])
const PLAINTEXT_PORTS = new Set([21, 23, 80, 110, 143, 161, 389, 8080])

const portInUse = computed(() => props.finding.remote_port || props.finding.local_port)

const encStatus = computed(() => {
  const port = portInUse.value
  if (!port) {
    return {
      state: 'unknown' as const,
      title: 'Encryption varies by connection',
      desc: 'This finding spans multiple connections. Whether data is encrypted depends on each individual session\'s port.',
      icon: 'question',
    }
  }
  if (ENCRYPTED_PORTS.has(port)) {
    return {
      state: 'encrypted' as const,
      title: `Traffic is encrypted  (port ${port} · TLS)`,
      desc: props.finding.category === 'cleartext_exfil'
        ? 'Unexpectedly, this connection appears to use TLS — the data may be hidden from network sniffers, but the destination is still malicious.'
        : 'The channel uses TLS — traffic content is hidden from passive network monitors. This makes interception harder but the malicious endpoint is unchanged. Encrypted C2 is harder to block via deep-packet inspection.',
      icon: 'lock',
    }
  }
  if (PLAINTEXT_PORTS.has(port)) {
    return {
      state: 'plaintext' as const,
      title: `Traffic is NOT encrypted  (port ${port} · cleartext)`,
      desc: 'Every byte sent and received — including credentials, commands, and file contents — is transmitted in plaintext. Any host on the same network segment can passively capture this traffic with a simple packet sniffer.',
      icon: 'unlock',
    }
  }
  return {
    state: 'unknown' as const,
    title: `Encryption unknown  (port ${port})`,
    desc: `Port ${port} has no standard encryption convention. Whether the session uses TLS depends on the application protocol. Treat as potentially plaintext.`,
    icon: 'question',
  }
})

// ── Per-category impact & action data ────────────────────────────────────────

interface Action  { label: string; desc: string }
interface Command { label: string; cmd: string }
interface Detail  { impacts: string[]; actions: Action[]; commands: Command[] }

function buildDetail(f: IntruderFinding): Detail {
  const pid  = f.pid.toString()
  const ip   = f.remote_ip
  const port = f.local_port || f.remote_port

  switch (f.category) {

    case 'known_bad_port':
      return {
        impacts: [
          'Remote attacker may have interactive shell access to this machine right now',
          'Files, keystrokes, clipboard, and screen contents may be accessible to the operator',
          ip ? `Traffic flows to ${ip} — likely a C2 server monitoring this session` : 'An external C2 server is coordinating this session',
          'Persistence mechanisms may already be in place — the connection may survive reboots',
          'Any credential typed since the process started should be considered compromised',
        ],
        actions: [
          { label: 'Kill the process immediately',        desc: 'Severing the connection is the first priority. Terminating the process cuts the attacker\'s access but does not remove the malware binary from disk.' },
          { label: 'Block the remote IP at the firewall', desc: 'Prevent reconnection even if the malware restarts. Use pf (macOS packet filter) to drop all traffic to the remote host.' },
          { label: 'Audit LaunchAgents for persistence',  desc: 'Malware typically installs a LaunchAgent or LaunchDaemon entry so it restarts on login or reboot. Check both user and system directories.' },
          { label: 'Run a full Defender scan',            desc: 'Use the Scanner pane to locate and quarantine the malware binary and any dropped payloads.' },
          { label: 'Rotate all credentials',              desc: 'Assume every password, API key, and SSH key accessible from this account since the infection time is compromised.' },
        ],
        commands: [
          { label: 'Full process path & binary',    cmd: `ps -p ${pid} -o args=` },
          { label: 'All network connections',       cmd: `lsof -p ${pid} -i` },
          { label: 'Verify code signature',         cmd: `codesign -dv --verbose=2 "$(ps -p ${pid} -o args= | awk '{print $1}')" 2>&1` },
          { label: 'Check LaunchAgents',            cmd: `ls -la ~/Library/LaunchAgents/ /Library/LaunchAgents/ /Library/LaunchDaemons/` },
          ...(ip ? [{ label: 'Block remote IP (pf)',    cmd: `echo 'block drop out quick proto tcp from any to ${ip}' | sudo pfctl -ef -` }] : []),
        ],
      }

    case 'suspicious_process':
      return {
        impacts: [
          'Execution from a temp or downloads directory bypasses macOS application allowlists',
          'Active network access indicates the process is calling home to a remote C2 server',
          'Malware launched from this pattern (browser download → execute) is a classic drive-by-download attack chain',
          'The binary may self-delete after installing persistence, leaving no file on disk',
          'A process from ~/Downloads making outbound connections is a strong malware indicator',
        ],
        actions: [
          { label: 'Kill the process',              desc: 'Stop any ongoing exfiltration or command execution immediately.' },
          { label: 'Preserve the binary',           desc: 'Copy the executable to a safe location before it can self-delete — you\'ll need it for forensic analysis.' },
          { label: 'Audit what it\'s connecting to', desc: 'Use lsof to reveal all remote hosts. This exposes the C2 infrastructure and helps blocklist it.' },
          { label: 'Inspect the binary',            desc: 'Use codesign and file to check whether it is a signed Apple binary, an unsigned app, or a raw executable.' },
          { label: 'Check LaunchAgents',            desc: 'The process may have installed a LaunchAgent to restart on login. Search for entries pointing to the binary\'s original location.' },
        ],
        commands: [
          { label: 'Full process path',             cmd: `ps -p ${pid} -o args=` },
          { label: 'All network connections',       cmd: `lsof -p ${pid} -i` },
          { label: 'Preserve binary before delete', cmd: `cp "$(ps -p ${pid} -o args= | awk '{print $1}')" ~/Desktop/preserved_$(date +%s) 2>/dev/null && echo Done` },
          { label: 'File type & signature',         cmd: `file "$(ps -p ${pid} -o args= | awk '{print $1}')"` },
          { label: 'Parent process (who launched it)', cmd: `ps -p $(ps -p ${pid} -o ppid= | tr -d ' ') -o user,pid,args` },
        ],
      }

    case 'lateral_movement':
      return {
        impacts: [
          'An attacker is using this machine as a stepping stone to compromise other LAN hosts',
          'Network-attached storage, other computers, printers, and routers on this network are all at risk',
          'Session tokens and NTLM hashes captured here may be replayed against other hosts without needing a password',
          'Once multiple machines are compromised, full network containment becomes significantly harder',
          'Internal services (databases, file shares, internal APIs) that trust this machine\'s IP are now exposed',
        ],
        actions: [
          { label: 'Isolate this machine from the network', desc: 'Disable WiFi and unplug Ethernet immediately to cut off lateral spread before other hosts are compromised.' },
          { label: 'Kill the process',                       desc: 'Terminate after network isolation to prevent final communication back to the attacker.' },
          { label: 'Alert your network administrator',       desc: 'Other devices must be audited for signs of compromise immediately. Share this machine\'s IP and the time of first activity.' },
          { label: 'Audit all LAN-connected devices',        desc: 'Check router DHCP logs and firewall logs for unusual connection attempts originating from this machine\'s IP address.' },
          { label: 'Rotate network credentials from a clean device', desc: 'WiFi passwords, admin accounts, and shared credentials should all be changed from a separate, uncompromised machine.' },
        ],
        commands: [
          { label: 'All local network connections',  cmd: `lsof -p ${pid} -i | grep -E '192\\.168\\.|10\\.|172\\.'` },
          { label: 'Full process info',              cmd: `ps -p ${pid} -o user,pid,ppid,args` },
          { label: 'Current LAN hosts (ARP table)',  cmd: `arp -a` },
          { label: 'Disable WiFi',                  cmd: `networksetup -setairportpower en0 off` },
          { label: 'Unique remote IPs used',         cmd: `lsof -p ${pid} -i | grep ESTABLISHED | awk '{print $9}' | sed 's/:.*//' | sort -u` },
        ],
      }

    case 'port_scan':
      return {
        impacts: [
          'This machine is actively probing external hosts — it may be a node in a distributed botnet',
          'Your IP address is likely being logged by every host being scanned, leading to blocklisting',
          'Outbound scanning may violate your ISP\'s terms of service and in some jurisdictions is illegal',
          'The scanning process may be selecting targets for follow-on exploitation or data theft',
          'If the process is a worm, it may be actively trying to infect other machines',
        ],
        actions: [
          { label: 'Kill the process',                      desc: 'Stop the scanning activity immediately.' },
          { label: 'Block all outbound traffic from this process', desc: 'Use the macOS Application Firewall or pf to prevent the process from making further outbound connections even if it restarts.' },
          { label: 'Identify and verify the binary',        desc: 'Determine if this is a security tool you installed intentionally. If not, it is malware.' },
          { label: 'Check who launched it',                 desc: 'The parent process may reveal how the scanner was deployed — via a browser, script, or another malicious process.' },
          { label: 'Run a full Defender scan',              desc: 'Port scanners are often deployed as part of larger malware packages.' },
        ],
        commands: [
          { label: 'All active connections',       cmd: `lsof -p ${pid} -i` },
          { label: 'Unique remote IPs',            cmd: `lsof -p ${pid} -i | grep ESTABLISHED | awk '{print $9}' | sed 's/:.*//' | sort -u | wc -l` },
          { label: 'Binary signature',             cmd: `codesign -dv "$(ps -p ${pid} -o args= | awk '{print $1}')" 2>&1` },
          { label: 'Parent process',               cmd: `ps -p $(ps -p ${pid} -o ppid= | tr -d ' ') -o user,pid,args` },
        ],
      }

    case 'backdoor_listener':
      return {
        impacts: [
          `Port ${port || '?'} is open on all network interfaces — any host that discovers it can attempt to connect`,
          'If this is a reverse shell listener, the first inbound connection may give an attacker a root or user shell',
          'The listener may be waiting for a specific secret (port-knock or password) before revealing itself',
          'Network scanners such as Shodan index internet-facing ports — this machine may already be catalogued',
          'Unauthorized listeners are a persistent backdoor that survives process restarts if paired with LaunchAgents',
        ],
        actions: [
          { label: 'Kill the listener process',            desc: 'Terminate the process to close the port immediately.' },
          { label: 'Block the port at the network layer',  desc: 'Even if the process restarts, a pf rule will prevent inbound connections to this port.' },
          { label: 'Audit who has already connected',      desc: 'Use lsof to check for ESTABLISHED inbound sessions — the attacker may already be inside.' },
          { label: 'Check for persistence',                desc: 'Search LaunchAgents and LaunchDaemons for entries that restart this listener automatically.' },
          { label: 'Rotate credentials',                   desc: 'If any inbound connection was established before discovery, assume all credentials on this machine are compromised.' },
        ],
        commands: [
          { label: 'Who has connected to this port',        cmd: `lsof -i :${port || '?'} -n -P | grep ESTABLISHED` },
          { label: 'Full process path',                     cmd: `ps -p ${pid} -o args=` },
          { label: 'Block port inbound (pf)',                cmd: `echo 'block drop in quick proto tcp from any to any port ${port || '?'}' | sudo pfctl -ef -` },
          { label: 'Search LaunchAgents for persistence',   cmd: `grep -rl "${f.process}" ~/Library/LaunchAgents/ /Library/LaunchAgents/ /Library/LaunchDaemons/ 2>/dev/null` },
        ],
      }

    case 'cleartext_exfil':
      return {
        impacts: [
          'All data — usernames, passwords, and file contents — travels in plaintext over the network',
          'Any host on the same network segment can capture every byte with a passive sniffer (no ARP poisoning required)',
          'Credentials captured over FTP or Telnet can be immediately replayed against SSH, email, and web services',
          'On shared or public networks (office WiFi, coffee shop), this data is trivially visible to all nearby users',
          ip ? `Files and credentials are being sent to ${ip} — the operator of that host has full access` : 'The remote host operator has visibility into all transferred content',
        ],
        actions: [
          { label: 'Kill the process',                            desc: 'Stop the cleartext transmission immediately.' },
          { label: 'Replace FTP/Telnet with encrypted alternatives', desc: 'Use SFTP (part of OpenSSH) or SCP instead of FTP. Use SSH instead of Telnet. Most servers support these today.' },
          { label: 'Audit what was transferred',                  desc: 'Review transfer logs on both the client and server side to understand what data was exposed.' },
          { label: 'Rotate credentials used with this service',   desc: 'Any username/password sent over this connection is compromised — change it on every service where it is reused.' },
          { label: 'Block FTP and Telnet outbound at the firewall', desc: 'Prevent future cleartext transfers by dropping outbound TCP on ports 21 and 23 system-wide.' },
        ],
        commands: [
          { label: 'All files opened by this process', cmd: `lsof -p ${pid}` },
          { label: 'Process details',                  cmd: `ps -p ${pid} -o user,pid,ppid,args` },
          { label: 'Block FTP outbound (port 21)',      cmd: `echo 'block drop out quick proto tcp from any to any port 21' | sudo pfctl -ef -` },
          { label: 'Block Telnet outbound (port 23)',   cmd: `echo 'block drop out quick proto tcp from any to any port 23' | sudo pfctl -ef -` },
        ],
      }

    default:
      return {
        impacts: ['Suspicious network activity detected — investigate immediately to determine scope.'],
        actions: [
          { label: 'Kill the process', desc: 'Terminate to stop the suspicious activity.' },
          { label: 'Investigate the process', desc: 'Check its full path, binary signature, and all open network connections.' },
        ],
        commands: [
          { label: 'Full process info',     cmd: `ps -p ${pid} -o user,pid,ppid,args` },
          { label: 'Network connections',   cmd: `lsof -p ${pid} -i` },
          { label: 'Code signature',        cmd: `codesign -dv "$(ps -p ${pid} -o args= | awk '{print $1}')" 2>&1` },
        ],
      }
  }
}

const detail = computed(() => buildDetail(props.finding))

// ── Severity config ──────────────────────────────────────────────────────────

const SEV_CONFIG = {
  critical: { label: 'Critical', color: 'var(--red)',    bg: 'var(--red-dim)',        border: 'rgba(248,81,73,0.3)' },
  high:     { label: 'High',     color: 'var(--orange)', bg: 'rgba(210,153,34,0.1)',  border: 'rgba(210,153,34,0.3)' },
  medium:   { label: 'Medium',   color: '#d29922',       bg: 'rgba(210,153,34,0.08)', border: 'rgba(210,153,34,0.22)' },
  low:      { label: 'Low',      color: 'var(--muted)',  bg: 'var(--surface-2)',      border: 'var(--border)' },
}

const sev = computed(() => {
  const k = props.finding.severity as keyof typeof SEV_CONFIG
  return SEV_CONFIG[k] ?? SEV_CONFIG.low
})

const categoryLabels: Record<string, string> = {
  known_bad_port:    'Malicious Port',
  suspicious_process:'Suspicious Process',
  lateral_movement:  'Lateral Movement',
  port_scan:         'Port Scan',
  backdoor_listener: 'Backdoor',
  cleartext_exfil:   'Cleartext Exfil',
}

// ── Kill action ──────────────────────────────────────────────────────────────

async function killProcess() {
  if (killing.value || killed.value) return
  killing.value = true
  killError.value = ''
  try {
    await invoke('kill_process', { pid: props.finding.pid })
    killed.value = true
  } catch (e) {
    killError.value = typeof e === 'string' ? e : String(e)
  } finally {
    killing.value = false
  }
}
</script>

<template>
  <aside class="intruder-detail">

    <!-- Header -->
    <div class="idetail-header">
      <div class="iheader-left">
        <span
          class="sev-badge"
          :style="{ color: sev.color, background: sev.bg, borderColor: sev.border }"
        >{{ sev.label }}</span>
        <div class="idetail-title">{{ finding.title }}</div>
      </div>
      <button class="close-btn" @click="$emit('close')">✕</button>
    </div>

    <!-- Scrollable body -->
    <div class="idetail-body">

      <!-- Meta chips -->
      <div class="meta-row">
        <span class="chip process">{{ finding.process }}</span>
        <span class="chip pid mono">PID {{ finding.pid }}</span>
        <span v-if="finding.remote_ip" class="chip mono">
          {{ finding.remote_ip }}<template v-if="finding.remote_port">:{{ finding.remote_port }}</template>
        </span>
        <span v-if="finding.local_port" class="chip mono">:{{ finding.local_port }} listening</span>
        <span class="chip cat">{{ categoryLabels[finding.category] ?? finding.category }}</span>
      </div>

      <div class="divider" />

      <!-- Encryption / transmission status -->
      <div :class="['enc-banner', encStatus.state]">
        <div class="enc-icon-wrap">
          <!-- Lock closed (encrypted) -->
          <svg v-if="encStatus.icon === 'lock'" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <rect x="4" y="9" width="12" height="9" rx="2"/>
            <path d="M7 9V6.5a3 3 0 0 1 6 0V9"/>
            <circle cx="10" cy="14" r="1.2" fill="currentColor" stroke="none"/>
          </svg>
          <!-- Lock open (plaintext) -->
          <svg v-else-if="encStatus.icon === 'unlock'" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <rect x="4" y="9" width="12" height="9" rx="2"/>
            <path d="M7 9V6.5a3 3 0 0 1 5.8-1"/>
            <line x1="13" y1="4" x2="14.5" y2="4"/>
            <circle cx="10" cy="14" r="1.2" fill="currentColor" stroke="none"/>
          </svg>
          <!-- Question (unknown) -->
          <svg v-else viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="10" cy="10" r="8"/>
            <path d="M7.5 7.5a2.5 2.5 0 0 1 4.5 1c0 1.5-2 2-2 3.5"/>
            <circle cx="10" cy="15" r="0.8" fill="currentColor" stroke="none"/>
          </svg>
        </div>
        <div class="enc-content">
          <div class="enc-title">{{ encStatus.title }}</div>
          <div class="enc-desc">{{ encStatus.desc }}</div>
        </div>
      </div>

      <div class="divider" />

      <!-- Potential Impact -->
      <div class="section">
        <div class="section-title impact-title">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
            <circle cx="8" cy="8" r="6.5" stroke="var(--red)"/>
            <line x1="8" y1="5" x2="8" y2="9" stroke="var(--red)"/>
            <circle cx="8" cy="11.5" r="0.7" fill="var(--red)" stroke="none"/>
          </svg>
          Potential Impact
        </div>
        <ul class="impact-list">
          <li v-for="(impact, i) in detail.impacts" :key="i">{{ impact }}</li>
        </ul>
      </div>

      <div class="divider" />

      <!-- Recommended Actions -->
      <div class="section">
        <div class="section-title fix-title">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
            <circle cx="8" cy="8" r="6.5" stroke="var(--green)"/>
            <polyline points="5 8 7 10.5 11 5.5" stroke="var(--green)"/>
          </svg>
          Recommended Actions
        </div>
        <ol class="fix-list">
          <li v-for="(action, i) in detail.actions" :key="i">
            <span class="step-num">{{ i + 1 }}</span>
            <div class="step-body">
              <div class="step-label">{{ action.label }}</div>
              <div class="step-desc">{{ action.desc }}</div>
            </div>
          </li>
        </ol>
      </div>

      <div class="divider" />

      <!-- Terminal Commands -->
      <div class="section">
        <div class="section-title">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
            <rect x="1.5" y="2.5" width="13" height="11" rx="1.5"/>
            <polyline points="4 6 6.5 8 4 10"/>
            <line x1="8" y1="10" x2="12" y2="10"/>
          </svg>
          Terminal Commands
        </div>
        <div class="cmd-list">
          <div v-for="cmd in detail.commands" :key="cmd.label" class="cmd-block">
            <div class="cmd-label">{{ cmd.label }}</div>
            <pre class="cmd-code">{{ cmd.cmd }}</pre>
          </div>
        </div>
      </div>

      <div class="divider" />

      <!-- Kill Process -->
      <div class="section kill-section">
        <div class="section-title kill-title">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
            <line x1="3" y1="3" x2="13" y2="13"/>
            <line x1="13" y1="3" x2="3" y2="13"/>
          </svg>
          Terminate Process
        </div>

        <div v-if="killed" class="kill-success">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:14px;height:14px">
            <circle cx="8" cy="8" r="6.5" stroke="var(--green)"/>
            <polyline points="5 8 7 10.5 11 5.5" stroke="var(--green)"/>
          </svg>
          Process {{ finding.pid }} terminated
        </div>
        <template v-else>
          <div v-if="killError" class="kill-error">{{ killError }}</div>
          <button
            class="kill-btn"
            :disabled="killing"
            @click="killProcess"
          >
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px">
              <line x1="3" y1="3" x2="13" y2="13"/>
              <line x1="13" y1="3" x2="3" y2="13"/>
            </svg>
            {{ killing ? 'Terminating…' : `Kill PID ${finding.pid}` }}
          </button>
          <div class="kill-hint">
            Sends SIGKILL — process stops immediately. The binary on disk is not removed; run a Defender scan to quarantine it.
          </div>
        </template>
      </div>

    </div>
  </aside>
</template>

<style scoped>
.intruder-detail {
  width: var(--panel-w);
  flex-shrink: 0;
  background: var(--surface);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* Header */
.idetail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
  gap: 10px;
  flex-shrink: 0;
}

.iheader-left {
  display: flex;
  flex-direction: column;
  gap: 7px;
  min-width: 0;
}

.sev-badge {
  display: inline-flex;
  align-items: center;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  padding: 2px 8px;
  border-radius: 5px;
  border: 1px solid;
  align-self: flex-start;
}

.idetail-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  line-height: 1.4;
}

.close-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 14px;
  padding: 4px;
  border-radius: 4px;
  flex-shrink: 0;
  margin-top: 2px;
  line-height: 1;
}
.close-btn:hover { color: var(--text); background: var(--surface-2); }

/* Body */
.idetail-body {
  flex: 1;
  overflow-y: auto;
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

/* Meta chips */
.meta-row { display: flex; flex-wrap: wrap; gap: 5px; }

.chip {
  font-size: 10px;
  padding: 2px 7px;
  border-radius: 4px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  color: var(--muted);
  white-space: nowrap;
}
.chip.process { color: var(--text); font-weight: 600; }
.chip.mono    { font-family: 'SF Mono', 'Menlo', monospace; }
.chip.cat {
  background: rgba(180,120,255,0.08);
  border-color: rgba(180,120,255,0.2);
  color: #b478ff;
}

.divider { height: 1px; background: var(--border); flex-shrink: 0; }

/* Encryption banner */
.enc-banner {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  padding: 12px 14px;
  border-radius: 10px;
  border: 1px solid;
}

.enc-banner.plaintext {
  background: var(--red-dim);
  border-color: rgba(248,81,73,0.25);
  color: var(--red);
}
.enc-banner.encrypted {
  background: rgba(210,153,34,0.10);
  border-color: rgba(210,153,34,0.28);
  color: #d29922;
}
.enc-banner.unknown {
  background: var(--surface-2);
  border-color: var(--border);
  color: var(--muted);
}

.enc-icon-wrap {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  margin-top: 1px;
}
.enc-icon-wrap svg { width: 20px; height: 20px; }

.enc-content { display: flex; flex-direction: column; gap: 4px; }

.enc-title {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.enc-desc {
  font-size: 11px;
  color: var(--text);
  line-height: 1.55;
  opacity: 0.9;
}

/* Sections */
.section { display: flex; flex-direction: column; gap: 10px; }

.section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
}

.impact-title { color: var(--red); }
.fix-title    { color: var(--green); }
.kill-title   { color: var(--red); }

/* Impact list */
.impact-list {
  list-style: none;
  padding: 10px 12px;
  margin: 0;
  background: var(--red-dim);
  border: 1px solid rgba(248,81,73,0.2);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.impact-list li {
  font-size: 12px;
  color: var(--text);
  line-height: 1.55;
  padding-left: 14px;
  position: relative;
}
.impact-list li::before {
  content: '›';
  position: absolute;
  left: 0;
  color: var(--red);
  font-weight: 700;
}

/* Fix list */
.fix-list {
  list-style: none;
  padding: 10px 12px;
  margin: 0;
  background: var(--green-dim);
  border: 1px solid rgba(63,185,80,0.2);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.fix-list li { display: flex; gap: 10px; align-items: flex-start; }

.step-num {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  background: var(--green);
  color: #000;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 9px;
  font-weight: 800;
  margin-top: 2px;
}

.step-body { display: flex; flex-direction: column; gap: 3px; }

.step-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
}
.step-desc {
  font-size: 11px;
  color: var(--muted);
  line-height: 1.55;
}

/* Commands */
.cmd-list { display: flex; flex-direction: column; gap: 8px; }

.cmd-block { display: flex; flex-direction: column; gap: 4px; }

.cmd-label {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
  font-weight: 500;
}

.cmd-code {
  font-family: 'SF Mono', 'Menlo', monospace;
  font-size: 11px;
  color: var(--text);
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 10px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.55;
  user-select: all;
}

/* Kill section */
.kill-section { padding-bottom: 8px; }

.kill-btn {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  justify-content: center;
  padding: 9px 16px;
  font-size: 12px;
  font-weight: 600;
  font-family: inherit;
  background: var(--red);
  border: 1px solid var(--red);
  color: #fff;
  border-radius: 7px;
  cursor: pointer;
  transition: filter 0.12s;
}
.kill-btn:hover:not(:disabled) { filter: brightness(1.12); }
.kill-btn:disabled { opacity: 0.55; cursor: not-allowed; }

.kill-success {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 12px;
  font-weight: 600;
  color: var(--green);
  padding: 8px 12px;
  background: var(--green-dim);
  border: 1px solid rgba(63,185,80,0.25);
  border-radius: 7px;
}

.kill-error {
  font-size: 11px;
  color: var(--red);
  margin-bottom: 6px;
}

.kill-hint {
  font-size: 11px;
  color: var(--muted);
  line-height: 1.5;
  margin-top: 7px;
}
</style>
