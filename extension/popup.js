const dot = document.getElementById('dot');
const statusText = document.getElementById('status-text');
const hint = document.getElementById('hint');

function setStatus(state) {
  dot.className = 'dot ' + state;
  if (state === 'connected') {
    statusText.textContent = 'Connected to NetScope';
    hint.textContent = 'Traffic is being captured. Open the Browser tab in NetScope to view it.';
  } else if (state === 'error') {
    statusText.textContent = 'NetScope not running';
    hint.textContent = 'Start the NetScope desktop app, then reload this extension.';
  } else {
    statusText.textContent = 'Connecting…';
  }
}

const ws = new WebSocket('ws://127.0.0.1:9922');
ws.onopen = () => setStatus('connected');
ws.onerror = ws.onclose = () => setStatus('error');
