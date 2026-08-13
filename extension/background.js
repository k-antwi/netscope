const WS_URL = 'ws://127.0.0.1:9922';
const MAX_QUEUE = 200;

let ws = null;
let wsReady = false;
const queue = [];
const pending = {}; // requestId → accumulated data

// ── WebSocket ────────────────────────────────────────────────────

function connect() {
  if (ws && (ws.readyState === WebSocket.CONNECTING || ws.readyState === WebSocket.OPEN)) return;

  ws = new WebSocket(WS_URL);

  ws.onopen = () => {
    wsReady = true;
    while (queue.length) ws.send(queue.shift());
  };

  ws.onclose = ws.onerror = () => {
    wsReady = false;
    setTimeout(connect, 3000);
  };
}

function send(data) {
  const msg = JSON.stringify(data);
  if (wsReady) {
    ws.send(msg);
  } else {
    queue.push(msg);
    if (queue.length > MAX_QUEUE) queue.shift();
  }
}

connect();

// ── Helpers ──────────────────────────────────────────────────────

function extractBody(requestBody) {
  if (!requestBody) return null;
  if (requestBody.formData) {
    return JSON.stringify(requestBody.formData);
  }
  if (requestBody.raw && requestBody.raw.length) {
    try {
      const decoder = new TextDecoder('utf-8');
      return requestBody.raw.map(chunk => decoder.decode(chunk.bytes)).join('');
    } catch {
      return '[binary]';
    }
  }
  return null;
}

function statusPhrase(statusLine, code) {
  const parts = (statusLine || '').split(' ');
  const phrase = parts.slice(2).join(' ').trim();
  if (phrase) return phrase;
  const defaults = {
    200: 'OK', 201: 'Created', 204: 'No Content',
    301: 'Moved Permanently', 302: 'Found', 304: 'Not Modified',
    400: 'Bad Request', 401: 'Unauthorized', 403: 'Forbidden',
    404: 'Not Found', 405: 'Method Not Allowed', 429: 'Too Many Requests',
    500: 'Internal Server Error', 502: 'Bad Gateway', 503: 'Service Unavailable',
  };
  return defaults[code] || '';
}

// ── webRequest listeners ─────────────────────────────────────────

// Phase 1: capture URL, method, body, start time
chrome.webRequest.onBeforeRequest.addListener(
  details => {
    pending[details.requestId] = {
      id: details.requestId,
      url: details.url,
      method: details.method,
      startTime: details.timeStamp,
      requestBody: extractBody(details.requestBody),
      requestHeaders: [],
      responseHeaders: [],
      status: 0,
      statusText: '',
      fromCache: false,
      initiator: details.initiator || '',
      tabUrl: '',
      error: null,
    };

    // Attach tab URL asynchronously
    if (details.tabId > 0) {
      chrome.tabs.get(details.tabId, tab => {
        if (!chrome.runtime.lastError && pending[details.requestId]) {
          pending[details.requestId].tabUrl = tab.url || '';
        }
      });
    }
  },
  { urls: ['<all_urls>'] },
  ['requestBody']
);

// Phase 2: capture request headers
chrome.webRequest.onSendHeaders.addListener(
  details => {
    if (pending[details.requestId]) {
      pending[details.requestId].requestHeaders = details.requestHeaders || [];
    }
  },
  { urls: ['<all_urls>'] },
  ['requestHeaders', 'extraHeaders']
);

// Phase 3: capture response status + headers
chrome.webRequest.onHeadersReceived.addListener(
  details => {
    const req = pending[details.requestId];
    if (req) {
      req.status = details.statusCode;
      req.statusText = statusPhrase(details.statusLine, details.statusCode);
      req.responseHeaders = details.responseHeaders || [];
    }
  },
  { urls: ['<all_urls>'] },
  ['responseHeaders', 'extraHeaders']
);

// Phase 4: request complete — compute timing and send
chrome.webRequest.onCompleted.addListener(
  details => {
    const req = pending[details.requestId];
    if (!req) return;
    send({
      id: req.id,
      url: req.url,
      method: req.method,
      status: req.status,
      statusText: req.statusText,
      requestHeaders: req.requestHeaders,
      responseHeaders: req.responseHeaders,
      requestBody: req.requestBody,
      timingMs: details.timeStamp - req.startTime,
      fromCache: details.fromCache,
      initiator: req.initiator,
      tabUrl: req.tabUrl,
      timestamp: req.startTime,
      error: null,
    });
    delete pending[details.requestId];
  },
  { urls: ['<all_urls>'] }
);

// Phase 4 (error path)
chrome.webRequest.onErrorOccurred.addListener(
  details => {
    const req = pending[details.requestId];
    if (!req) return;
    send({
      id: req.id,
      url: req.url,
      method: req.method,
      status: 0,
      statusText: 'Network Error',
      requestHeaders: req.requestHeaders,
      responseHeaders: [],
      requestBody: req.requestBody,
      timingMs: details.timeStamp - req.startTime,
      fromCache: false,
      initiator: req.initiator,
      tabUrl: req.tabUrl,
      timestamp: req.startTime,
      error: details.error,
    });
    delete pending[details.requestId];
  },
  { urls: ['<all_urls>'] }
);
