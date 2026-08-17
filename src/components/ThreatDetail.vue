<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ThreatItem } from '../types'

const props = defineProps<{ threat: ThreatItem; isNeutralized: boolean }>()
const emit = defineEmits<{ close: []; neutralized: [path: string] }>()

const neutralizing   = ref(false)
const neutralizeError = ref('')
const localNeutralized = ref(props.isNeutralized)

watch(() => props.threat.path, () => {
  neutralizing.value    = false
  neutralizeError.value = ''
  localNeutralized.value = props.isNeutralized
})
watch(() => props.isNeutralized, v => { localNeutralized.value = v })

async function doNeutralize() {
  if (neutralizing.value || localNeutralized.value) return
  neutralizing.value    = true
  neutralizeError.value = ''
  try {
    await invoke('neutralize_threat', { path: props.threat.path })
    localNeutralized.value = true
    emit('neutralized', props.threat.path)
  } catch (e) {
    neutralizeError.value = typeof e === 'string' ? e : String(e)
  } finally {
    neutralizing.value = false
  }
}

// ── Execute action modal ─────────────────────────────────────────────────────

interface ExecResult { stdout: string; stderr: string; success: boolean }

const execOpen    = ref(false)
const execLabel   = ref('')
const execCmd     = ref('')
const execRunning = ref(false)
const execResult  = ref<ExecResult | null>(null)

async function executeAction(label: string, cmd: string) {
  execLabel.value   = label
  execCmd.value     = cmd
  execResult.value  = null
  execRunning.value = true
  execOpen.value    = true
  try {
    execResult.value = await invoke<ExecResult>('run_command', { cmd })
  } catch (e) {
    execResult.value = { stdout: '', stderr: typeof e === 'string' ? e : String(e), success: false }
  } finally {
    execRunning.value = false
  }
}

function closeExec() { execOpen.value = false }

// ── Per-type content ─────────────────────────────────────────────────────────

interface Action  { label: string; desc: string; cmd?: string; isNeutralize?: boolean }
interface Command { label: string; cmd: string }
interface Content { description: string; networkRisks: string[]; actions: Action[]; commands: Command[] }

function buildContent(t: ThreatItem): Content {
  const p = t.path
  const n = t.name

  switch (t.threat_type) {

    case 'suspicious_launchagent':
      return {
        description: 'A LaunchAgent is a macOS persistence mechanism that runs automatically every time you log in. A suspicious LaunchAgent can maintain a persistent connection to a remote server, exfiltrate data on a schedule, or give an attacker recurring access to this machine — surviving reboots and operating invisibly in the background without any user interaction.',
        networkRisks: [
          'Survives reboots — an attacker retains access even after the machine restarts',
          'Can beacon to a command-and-control server on a fixed schedule without user interaction',
          'May silently download and install additional malware payloads over time',
          'Can exfiltrate files, keystrokes, and saved credentials in the background',
          'Persistent access makes it difficult to determine how long the machine has been compromised',
        ],
        actions: [
          { label: 'Quarantine the agent',          desc: 'Move the LaunchAgent plist to quarantine — prevents it from loading on next login.', isNeutralize: true },
          { label: 'Unload from this session now',  desc: 'Stop the agent from running immediately without waiting for a reboot.', cmd: `launchctl unload "${p}" 2>&1` },
          { label: 'Inspect the plist contents',   desc: 'Read the plist to identify what binary or script it runs and what network addresses it contacts.', cmd: `cat "${p}" 2>&1` },
          { label: 'Find the binary it references', desc: 'Extract the program path from the plist so you can locate and remove the underlying binary.', cmd: `/usr/libexec/PlistBuddy -c "Print :ProgramArguments" "${p}" 2>&1` },
          { label: 'Run a full Defender scan',      desc: 'Use the Scanner pane to locate and quarantine the binary or script this LaunchAgent executes.' },
        ],
        commands: [
          { label: 'View plist contents',       cmd: `cat "${p}"` },
          { label: 'Check if loaded',           cmd: `launchctl list | grep "${n}"` },
          { label: 'Unload the agent',          cmd: `launchctl unload "${p}"` },
          { label: 'Find binary it references', cmd: `/usr/libexec/PlistBuddy -c "Print :ProgramArguments" "${p}" 2>&1` },
        ],
      }

    case 'suspicious_script':
      return {
        description: 'A suspicious script (shell, Python, Ruby, Perl, or similar) can execute arbitrary system commands, download payloads, modify system files, and establish outbound network connections — all without requiring a compiled binary. Scripts are easy to disguise as legitimate tools and can be updated remotely by a command-and-control server without deploying a new file.',
        networkRisks: [
          'Can make outbound requests to download additional malware stages at any time',
          'May contain embedded credentials or API keys that could be exfiltrated when the script runs',
          'Can pivot to other machines on the same network via SSH, SCP, or custom sockets',
          'Easy to modify remotely — a C2 server can change what it does without redeploying',
          'Running as your user account gives it access to all your files, browser data, and cloud credentials',
        ],
        actions: [
          { label: 'Quarantine the script',                desc: 'Move the script to quarantine to prevent it from being executed again.', isNeutralize: true },
          { label: 'Inspect the script contents',          desc: 'Read the script to understand what it does — look for curl, wget, hardcoded IPs, or encoded payloads.', cmd: `cat "${p}" 2>&1` },
          { label: 'Scan for embedded network targets',    desc: 'Search for network-related commands or hardcoded domains inside the script body.', cmd: `grep -iEn 'curl|wget|http|nc |ncat|python.*socket|ssh|scp|rsync|base64' "${p}" 2>&1 | head -40` },
          { label: 'Check file permissions',               desc: 'See whether the script is executable and who owns it — root-owned scripts in user directories are especially suspicious.', cmd: `ls -la "${p}" 2>&1` },
          { label: 'Check if currently running',           desc: 'Determine whether the script is already active inside a running process.', cmd: `ps aux | grep "${n}" | grep -v grep 2>&1` },
        ],
        commands: [
          { label: 'View script contents',            cmd: `cat "${p}"` },
          { label: 'Search for network references',   cmd: `grep -iEn 'curl|wget|http|nc |ssh' "${p}" | head -20` },
          { label: 'File permissions',                cmd: `ls -la "${p}"` },
          { label: 'Check if running',                cmd: `ps aux | grep "${n}" | grep -v grep` },
        ],
      }

    case 'suspicious_executable':
      return {
        description: 'A suspicious executable binary outside standard system paths can perform any operation your user account is permitted to do — including establishing network connections, modifying files, installing persistence mechanisms, and exfiltrating data. Unsigned or improperly signed binaries in user-writable locations like Downloads are a primary malware delivery vector on macOS.',
        networkRisks: [
          'Can establish persistent outbound connections to command-and-control servers',
          'May already be exfiltrating files, keystrokes, or saved passwords right now',
          'Unsigned or ad-hoc signed binaries can bypass macOS Gatekeeper and quarantine checks',
          'Can silently download and install additional stages including kernel extensions',
          'A binary in Downloads may be part of a drive-by download — the full infection chain may still be unknown',
        ],
        actions: [
          { label: 'Quarantine the binary',              desc: 'Quarantine the executable to prevent it from running again.', isNeutralize: true },
          { label: 'Verify code signature',              desc: 'Check whether the binary is signed by Apple, a known developer, ad-hoc signed, or entirely unsigned.', cmd: `codesign -dv --verbose=2 "${p}" 2>&1` },
          { label: 'Identify exact binary type',         desc: 'Determine the architecture, format, and type — or confirm it is a script disguised as an executable.', cmd: `file "${p}" 2>&1` },
          { label: 'Check if currently running',         desc: 'Determine whether the binary is already active as a process.', cmd: `ps aux | grep "${n}" | grep -v grep 2>&1` },
          { label: 'Inspect open network connections',   desc: 'If the binary is running, see every remote host it is currently connected to.', cmd: `lsof -p $(pgrep -f "${n}" | head -1) -i 2>/dev/null || echo "Process not running or not found"` },
        ],
        commands: [
          { label: 'Code signature',       cmd: `codesign -dv --verbose=2 "${p}" 2>&1` },
          { label: 'Binary type (file)',   cmd: `file "${p}"` },
          { label: 'Check if running',     cmd: `ps aux | grep "${n}" | grep -v grep` },
          { label: 'Network connections',  cmd: `lsof -p $(pgrep -f "${n}" | head -1) -i 2>/dev/null` },
          { label: 'Check quarantine flag',cmd: `xattr -p com.apple.quarantine "${p}" 2>&1` },
        ],
      }

    case 'hidden_executable':
      return {
        description: 'A hidden executable (prefixed with a dot or stored in a concealed path) deliberately avoids detection by Finder and standard directory listings. Legitimate software has no reason to hide its binary — deliberate concealment strongly indicates malicious intent and suggests the author anticipated forensic investigation and actively tried to evade it.',
        networkRisks: [
          'Concealment indicates sophistication — assume the system is fully compromised until proven otherwise',
          'May have already established covert, long-running network channels operating in the background',
          'Hidden executables are frequently paired with LaunchAgents to auto-restart after termination',
          'The binary may have been planted remotely and the initial access vector is still unknown',
          'All credentials entered on this machine since the file was created should be considered compromised',
        ],
        actions: [
          { label: 'Quarantine the hidden binary',      desc: 'Immediately quarantine the file to prevent it from running again.', isNeutralize: true },
          { label: 'Verify code signature',             desc: 'Unsigned or ad-hoc signed hidden binaries are almost never legitimate software.', cmd: `codesign -dv --verbose=2 "${p}" 2>&1` },
          { label: 'Check if it is running now',        desc: 'If active, identify its network connections before terminating so you can trace the C2 infrastructure.', cmd: `ps aux | grep "${n}" | grep -v grep 2>&1` },
          { label: 'Search for related hidden files',   desc: 'Hidden malware often drops additional hidden files nearby — search the parent directory.', cmd: `find "$(dirname "${p}")" -name ".*" -type f 2>/dev/null` },
          { label: 'Check for LaunchAgent persistence', desc: 'Look for LaunchAgent plists that reference this binary and will restart it after termination or reboot.', cmd: `grep -rl "${n}" ~/Library/LaunchAgents/ /Library/LaunchAgents/ /Library/LaunchDaemons/ 2>/dev/null || echo "No persistence entries found"` },
        ],
        commands: [
          { label: 'Code signature',              cmd: `codesign -dv --verbose=2 "${p}" 2>&1` },
          { label: 'Binary type',                 cmd: `file "${p}"` },
          { label: 'Check if running',            cmd: `ps aux | grep "${n}" | grep -v grep` },
          { label: 'Find related hidden files',   cmd: `find "$(dirname "${p}")" -name ".*" -type f 2>/dev/null` },
          { label: 'Search for persistence',      cmd: `grep -rl "${n}" ~/Library/LaunchAgents/ /Library/LaunchAgents/ 2>/dev/null` },
        ],
      }

    default:
      return {
        description: 'This file was flagged as a potential threat based on its location, type, or behavioral characteristics. It may pose a risk to network security or system integrity.',
        networkRisks: [
          'May establish unauthorized outbound connections to external servers',
          'Could exfiltrate sensitive data including credentials and personal files',
          'May download and install additional malware components',
        ],
        actions: [
          { label: 'Quarantine the file',              desc: 'Move the file to quarantine to prevent it from being executed.', isNeutralize: true },
          { label: 'Inspect file type and signature',  desc: 'Determine the kind of file this is and whether it carries a valid code signature.', cmd: `file "${p}" 2>&1 && codesign -dv "${p}" 2>&1` },
          { label: 'Check if currently running',       desc: 'Determine whether the file is already executing as a process.', cmd: `ps aux | grep "${n}" | grep -v grep 2>&1` },
        ],
        commands: [
          { label: 'File type',        cmd: `file "${p}"` },
          { label: 'Code signature',   cmd: `codesign -dv "${p}" 2>&1` },
          { label: 'Check if running', cmd: `ps aux | grep "${n}" | grep -v grep` },
        ],
      }
  }
}

const content = computed(() => buildContent(props.threat))

// ── Severity config ──────────────────────────────────────────────────────────

const SEV_CONFIG = {
  critical: { label: 'Critical', color: 'var(--red)',    bg: 'var(--red-dim)',        border: 'rgba(248,81,73,0.3)' },
  high:     { label: 'High',     color: 'var(--orange)', bg: 'rgba(210,153,34,0.1)',  border: 'rgba(210,153,34,0.3)' },
  medium:   { label: 'Medium',   color: '#d29922',       bg: 'rgba(210,153,34,0.08)', border: 'rgba(210,153,34,0.22)' },
  low:      { label: 'Low',      color: 'var(--muted)',  bg: 'var(--surface-2)',      border: 'var(--border)' },
}
const sev = computed(() => SEV_CONFIG[props.threat.severity as keyof typeof SEV_CONFIG] ?? SEV_CONFIG.low)

const TYPE_LABELS: Record<string, string> = {
  suspicious_launchagent: 'Launch Agent',
  suspicious_script:      'Script',
  suspicious_executable:  'Executable',
  hidden_executable:      'Hidden File',
}
function typeLabel(t: string) { return TYPE_LABELS[t] ?? t }

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB']
  let v = bytes / 1024, u = 0
  while (v >= 1024 && u < units.length - 1) { v /= 1024; u++ }
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} ${units[u]}`
}

function formatModified(ts: number | null): string {
  if (!ts) return 'Unknown'
  return new Date(ts * 1000).toLocaleString([], { month: 'short', day: 'numeric', year: 'numeric', hour: '2-digit', minute: '2-digit' })
}
</script>

<template>
  <aside class="threat-detail">

    <!-- Header -->
    <div class="tdetail-header">
      <div class="theader-left">
        <span class="sev-badge" :style="{ color: sev.color, background: sev.bg, borderColor: sev.border }">
          {{ sev.label }}
        </span>
        <div class="tdetail-name">{{ threat.name }}</div>
        <div class="tdetail-reason">{{ threat.reason }}</div>
      </div>
      <button class="close-btn" @click="$emit('close')">✕</button>
    </div>

    <!-- Scrollable body -->
    <div class="tdetail-body">

      <!-- Meta chips + path -->
      <div class="meta-row">
        <span class="chip type-chip">{{ typeLabel(threat.threat_type) }}</span>
        <span class="chip mono">{{ formatSize(threat.size) }}</span>
        <span v-if="threat.modified" class="chip">{{ formatModified(threat.modified) }}</span>
      </div>
      <div class="path-display mono">{{ threat.path }}</div>

      <div class="divider" />

      <!-- What this threat does -->
      <div class="section">
        <div class="section-title">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
            <circle cx="8" cy="8" r="6.5"/>
            <line x1="8" y1="5.5" x2="8" y2="8.5"/>
            <circle cx="8" cy="11" r="0.7" fill="currentColor" stroke="none"/>
          </svg>
          What This Threat Does
        </div>
        <p class="threat-desc">{{ content.description }}</p>
      </div>

      <div class="divider" />

      <!-- Network risk -->
      <div class="section">
        <div class="section-title risk-title">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
            <circle cx="8" cy="8" r="6.5" stroke="var(--red)"/>
            <line x1="8" y1="5" x2="8" y2="9" stroke="var(--red)"/>
            <circle cx="8" cy="11.5" r="0.7" fill="var(--red)" stroke="none"/>
          </svg>
          Network Risk
        </div>
        <ul class="risk-list">
          <li v-for="(risk, i) in content.networkRisks" :key="i">{{ risk }}</li>
        </ul>
      </div>

      <div class="divider" />

      <!-- Recommended actions -->
      <div class="section">
        <div class="section-title fix-title">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
            <circle cx="8" cy="8" r="6.5" stroke="var(--green)"/>
            <polyline points="5 8 7 10.5 11 5.5" stroke="var(--green)"/>
          </svg>
          Recommended Actions
        </div>
        <ol class="fix-list">
          <li v-for="(action, i) in content.actions" :key="i">
            <span class="step-num">{{ i + 1 }}</span>
            <div class="step-body">
              <div class="step-label">{{ action.label }}</div>
              <div class="step-desc">{{ action.desc }}</div>
              <div v-if="action.isNeutralize && neutralizeError" class="step-error">{{ neutralizeError }}</div>
            </div>
            <!-- Quarantine button -->
            <button
              v-if="action.isNeutralize"
              class="exec-btn quarantine-btn"
              :class="{ done: localNeutralized }"
              :disabled="localNeutralized || neutralizing"
              @click="doNeutralize"
            >
              <svg v-if="localNeutralized" viewBox="0 0 10 10" fill="currentColor" style="width:8px;height:8px;flex-shrink:0">
                <polyline points="1.5,5 4,7.5 8.5,2.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <svg v-else viewBox="0 0 10 12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:8px;height:8px;flex-shrink:0">
                <path d="M5 1L1 2.5v4c0 2.5 1.8 4.3 4 4.7C7.2 10.8 9 9 9 6.5V2.5L5 1z"/>
              </svg>
              {{ localNeutralized ? 'Quarantined' : neutralizing ? '…' : 'Quarantine' }}
            </button>
            <!-- Run command button -->
            <button
              v-else-if="action.cmd"
              class="exec-btn"
              @click="executeAction(action.label, action.cmd!)"
              title="Run this command"
            >
              <svg viewBox="0 0 10 12" fill="currentColor" style="width:8px;height:8px;flex-shrink:0">
                <polygon points="0,0 10,6 0,12"/>
              </svg>
              Run
            </button>
          </li>
        </ol>
      </div>

      <div class="divider" />

      <!-- Terminal commands -->
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
          <div v-for="cmd in content.commands" :key="cmd.label" class="cmd-block">
            <div class="cmd-label-row">{{ cmd.label }}</div>
            <pre class="cmd-code">{{ cmd.cmd }}</pre>
          </div>
        </div>
      </div>

    </div>
  </aside>

  <!-- Execution modal -->
  <Teleport to="body">
    <div v-if="execOpen" class="exec-overlay" @click.self="closeExec">
      <div class="exec-modal" role="dialog" :aria-label="execLabel">

        <div class="exec-modal-header">
          <div class="exec-modal-title">{{ execLabel }}</div>
          <button class="close-btn" @click="closeExec">✕</button>
        </div>

        <div class="exec-modal-body">
          <div>
            <div class="exec-section-label">Command</div>
            <pre class="exec-code">{{ execCmd }}</pre>
          </div>

          <div v-if="execRunning" class="exec-status running">
            <span class="exec-spinner"/>
            Running…
          </div>

          <template v-else-if="execResult">
            <div :class="['exec-status', execResult.success ? 'success' : 'error']">
              <svg v-if="execResult.success" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
                <circle cx="8" cy="8" r="6.5" stroke="var(--green)"/>
                <polyline points="5 8 7 10.5 11 5.5" stroke="var(--green)"/>
              </svg>
              <svg v-else viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
                <circle cx="8" cy="8" r="6.5" stroke="var(--red)"/>
                <line x1="5.5" y1="5.5" x2="10.5" y2="10.5" stroke="var(--red)"/>
                <line x1="10.5" y1="5.5" x2="5.5" y2="10.5" stroke="var(--red)"/>
              </svg>
              {{ execResult.success ? 'Completed' : 'Error' }}
            </div>

            <div v-if="(execResult.stdout + execResult.stderr).trim()">
              <div class="exec-section-label">Output</div>
              <pre class="exec-output">{{ (execResult.stdout + (execResult.stderr ? '\n' + execResult.stderr : '')).trim() }}</pre>
            </div>
          </template>
        </div>

        <div class="exec-modal-footer">
          <button class="exec-close-btn" @click="closeExec">Close</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.threat-detail {
  width: var(--panel-w);
  flex-shrink: 0;
  background: var(--surface);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* Header */
.tdetail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
  gap: 10px;
  flex-shrink: 0;
}

.theader-left {
  display: flex;
  flex-direction: column;
  gap: 6px;
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

.tdetail-name {
  font-size: 13px;
  font-weight: 700;
  color: var(--text);
  line-height: 1.35;
  word-break: break-all;
}

.tdetail-reason {
  font-size: 11px;
  color: var(--muted);
  line-height: 1.45;
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
.tdetail-body {
  flex: 1;
  overflow-y: auto;
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

/* Meta row */
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
.chip.mono { font-family: 'SF Mono', 'Menlo', monospace; }
.chip.type-chip {
  background: rgba(180, 120, 255, 0.08);
  border-color: rgba(180, 120, 255, 0.22);
  color: #b478ff;
}

.path-display {
  font-family: 'SF Mono', 'Menlo', monospace;
  font-size: 10px;
  color: var(--muted);
  line-height: 1.5;
  word-break: break-all;
  padding: 6px 8px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
}

.divider { height: 1px; background: var(--border); flex-shrink: 0; }

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
.risk-title { color: var(--red); }
.fix-title  { color: var(--green); }

/* Threat description */
.threat-desc {
  font-size: 12px;
  color: var(--text);
  line-height: 1.65;
  margin: 0;
  padding: 10px 12px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 8px;
}

/* Risk list */
.risk-list {
  list-style: none;
  padding: 10px 12px;
  margin: 0;
  background: var(--red-dim);
  border: 1px solid rgba(248, 81, 73, 0.2);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.risk-list li {
  font-size: 12px;
  color: var(--text);
  line-height: 1.55;
  padding-left: 14px;
  position: relative;
}
.risk-list li::before {
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
  border: 1px solid rgba(63, 185, 80, 0.2);
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

.step-body { flex: 1; display: flex; flex-direction: column; gap: 3px; }

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
.step-error {
  font-size: 11px;
  color: var(--red);
  margin-top: 2px;
}

/* Execute button (shared) */
.exec-btn {
  flex-shrink: 0;
  align-self: flex-start;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  font-size: 10px;
  font-weight: 600;
  font-family: inherit;
  background: rgba(63, 185, 80, 0.1);
  border: 1px solid rgba(63, 185, 80, 0.28);
  color: var(--green);
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.12s, border-color 0.12s;
  white-space: nowrap;
  margin-top: 2px;
}
.exec-btn:hover {
  background: rgba(63, 185, 80, 0.2);
  border-color: rgba(63, 185, 80, 0.5);
}

/* Quarantine variant */
.quarantine-btn {
  background: rgba(248, 81, 73, 0.1);
  border-color: rgba(248, 81, 73, 0.3);
  color: var(--red);
}
.quarantine-btn:hover:not(:disabled) {
  background: rgba(248, 81, 73, 0.2);
  border-color: rgba(248, 81, 73, 0.5);
}
.quarantine-btn.done {
  background: rgba(63, 185, 80, 0.1);
  border-color: rgba(63, 185, 80, 0.3);
  color: var(--green);
  cursor: default;
}
.exec-btn:disabled { opacity: 0.55; cursor: not-allowed; }

/* Commands */
.cmd-list { display: flex; flex-direction: column; gap: 8px; }
.cmd-block { display: flex; flex-direction: column; gap: 4px; }
.cmd-label-row {
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

/* ── Execution modal ── */
.exec-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
}

.exec-modal {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  width: 540px;
  max-width: 100%;
  max-height: calc(100vh - 80px);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.4);
}

.exec-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 13px 16px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.exec-modal-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  min-width: 0;
}

.exec-modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.exec-section-label {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.07em;
  color: var(--muted);
  font-weight: 600;
  margin-bottom: 5px;
}

.exec-code {
  font-family: 'SF Mono', 'Menlo', monospace;
  font-size: 11px;
  color: var(--text);
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 9px 11px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.55;
}

.exec-output {
  font-family: 'SF Mono', 'Menlo', monospace;
  font-size: 11px;
  color: var(--text);
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 9px 11px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.55;
  max-height: 220px;
  overflow-y: auto;
}

.exec-status {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 12px;
  font-weight: 600;
  padding: 8px 12px;
  border-radius: 7px;
  border: 1px solid;
}
.exec-status.running {
  color: var(--muted);
  background: var(--surface-2);
  border-color: var(--border);
}
.exec-status.success {
  color: var(--green);
  background: var(--green-dim);
  border-color: rgba(63, 185, 80, 0.25);
}
.exec-status.error {
  color: var(--red);
  background: var(--red-dim);
  border-color: rgba(248, 81, 73, 0.25);
}

.exec-spinner {
  width: 13px;
  height: 13px;
  border: 2px solid var(--border);
  border-top-color: var(--muted);
  border-radius: 50%;
  animation: exec-spin 0.75s linear infinite;
  flex-shrink: 0;
}
@keyframes exec-spin { to { transform: rotate(360deg); } }

.exec-modal-footer {
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: flex-end;
  flex-shrink: 0;
}
.exec-close-btn {
  padding: 7px 18px;
  font-size: 12px;
  font-weight: 600;
  font-family: inherit;
  background: var(--surface-2);
  border: 1px solid var(--border);
  color: var(--text);
  border-radius: 7px;
  cursor: pointer;
  transition: background 0.12s;
}
.exec-close-btn:hover { background: var(--border); }
</style>
