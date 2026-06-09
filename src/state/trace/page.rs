//! Web UI 页面：ordinary-http handler

/// 链路追踪 Web UI 页面
///
/// 返回内嵌的 HTML 页面，前端通过 afast-code 生成的 JS 客户端与 binary handler 通信。
/// SSE 地址从 AppState 配置注入。
#[afast::get(desc("链路追踪 Web UI"), no_trace)]
pub async fn tracing_page(
    afast::State(state): afast::State<crate::AppState>,
) -> afast::Result<afast::Html> {
    let sse_path = format!("{}/events", state.tracing.config().url);
    let html = UI.replace("__SSE_PATH__", &sse_path);
    Ok(afast::Html(html))
}

const UI: &str = r#"<!DOCTYPE html>
<html lang="zh-CN" data-theme="dark">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'><defs><linearGradient id='g' x1='0%25' y1='0%25' x2='100%25' y2='100%25'><stop offset='0%25' stop-color='%2322d3ee'/><stop offset='100%25' stop-color='%236366f1'/></linearGradient></defs><rect width='32' height='32' rx='6' fill='%230f172a'/><path d='M18 5 L10 17 L15 17 L14 27 L22 15 L17 15 Z' stroke='url(%23g)' stroke-width='1.2' stroke-linejoin='round' fill='url(%23g)' fill-opacity='0.15'/></svg>">
<title>Tracing - 链路追踪</title>
<style>
:root, [data-theme="light"] {
    --bg: #ffffff; --surface: #f8f9fa; --surface2: #e9ecef; --border: #dee2e6;
    --text: #212529; --text2: #6c757d; --text3: #adb5bd;
    --blue: #0d6efd; --green: #198754; --red: #dc3545;
    --orange: #ffc107; --purple: #667eea;
    --hover: rgba(13,110,253,0.04); --spinner-bg: #dee2e6;
    --gantt-track: rgba(0,0,0,0.03); --shadow: rgba(0,0,0,0.08);
    --overlay: rgba(0,0,0,0.4); --accent-bg: #e7f1ff;
}
[data-theme="dark"] {
    --bg: #111113; --surface: #1a1a1d; --surface2: #222225; --border: #2c2e33;
    --text: #e0e0e0; --text2: #909296; --text3: #5c5f66;
    --blue: #339af0; --green: #40c057; --red: #fa5252;
    --orange: #fcc419; --purple: #667eea;
    --hover: rgba(51,154,240,0.06); --spinner-bg: #2c2e33;
    --gantt-track: rgba(255,255,255,0.03); --shadow: rgba(0,0,0,0.3);
    --overlay: rgba(0,0,0,0.6); --accent-bg: #1a2a3a;
}
* { margin:0; padding:0; box-sizing:border-box; }
body { font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif; background:var(--bg); color:var(--text); min-height:100vh; transition:background 0.2s,color 0.2s; }
.container { max-width:1400px; margin:0 auto; padding:20px; }
header { display:flex; align-items:center; justify-content:space-between; padding:16px 0; border-bottom:1px solid var(--border); margin-bottom:20px; }
header h1 { font-size:24px; font-weight:600; }
header h1 span { color:var(--blue); }
.header-right { display:flex; align-items:center; gap:12px; }

/* Theme Toggle */
.theme-toggle { background:var(--surface); border:1px solid var(--border); border-radius:6px; padding:6px 10px; cursor:pointer; font-size:17px; color:var(--text); display:flex; align-items:center; gap:4px; transition:border-color 0.2s; }
.theme-toggle:hover { border-color:var(--blue); }

/* Real-time Toggle */
.realtime-toggle { background:var(--surface); border:1px solid var(--border); border-radius:6px; padding:6px 12px; cursor:pointer; font-size:14px; color:var(--text2); display:flex; align-items:center; gap:6px; transition:all 0.2s; position:relative; }
.realtime-toggle:hover { border-color:var(--blue); }
.realtime-toggle.active { border-color:var(--green); color:var(--green); }
.realtime-dot { width:8px; height:8px; border-radius:50%; background:var(--text3); }
.realtime-toggle.active .realtime-dot { background:var(--green); animation:blink 1.5s infinite; }
@keyframes blink { 0%,100% { opacity:1; } 50% { opacity:0.3; } }
.realtime-count { background:var(--blue); color:#fff; font-size:11px; padding:1px 6px; border-radius:10px; min-width:18px; text-align:center; display:none; }
.realtime-count.show { display:inline-block; }

/* Stats Cards */
.stats { display:grid; grid-template-columns:repeat(auto-fit,minmax(160px,1fr)); gap:12px; margin-bottom:20px; }
.stat-card { background:var(--surface); border:1px solid var(--border); border-radius:8px; padding:16px; transition:background 0.2s,border-color 0.2s; }
.stat-card .label { font-size:15px; color:var(--text2); margin-bottom:4px; }
.stat-card .value { font-size:32px; font-weight:700; }
.stat-card .value.ok { color:var(--green); }
.stat-card .value.err { color:var(--red); }

/* Filters */
.filters { display:flex; gap:10px; flex-wrap:wrap; margin-bottom:16px; align-items:center; }
.filters select, .filters input, .filters button { background:var(--surface); border:1px solid var(--border); border-radius:6px; color:var(--text); padding:8px 12px; font-size:16px; outline:none; transition:border-color 0.2s; line-height:1.4; height:40px; }
.filters select:focus, .filters input:focus { border-color:var(--blue); }
.filters button { cursor:pointer; background:var(--blue); border-color:var(--blue); color:#fff; font-weight:500; }
.filters button:hover { opacity:0.9; }
.filters button.secondary { background:var(--surface); border-color:var(--border); color:var(--text); }

/* Table */
.table-wrap { background:var(--surface); border:1px solid var(--border); border-radius:8px; overflow:hidden; transition:background 0.2s,border-color 0.2s; }
table { width:100%; border-collapse:collapse; }
th { text-align:left; padding:12px 16px; font-size:15px; color:var(--text2); font-weight:500; border-bottom:1px solid var(--border); background:var(--surface2); }
td { padding:10px 16px; font-size:16px; border-bottom:1px solid var(--border); }
tr:last-child td { border-bottom:none; }
tr:hover { background:var(--hover); }
tr.clickable { cursor:pointer; }
.badge { display:inline-block; padding:2px 10px; border-radius:10px; font-size:14px; font-weight:600; letter-spacing:0.3px; }
.badge.ok { background:var(--accent-bg); color:var(--green); }
.badge.error { background:var(--danger-bg, rgba(250,82,82,0.15)); color:var(--red); }
.badge.call { background:linear-gradient(135deg, #667eea 0%, #764ba2 100%); color:#fff; }
.badge.long { background:linear-gradient(135deg, #f093fb 0%, #f5576c 100%); color:#fff; }
.badge.get { background:var(--green); color:#fff; }
.badge.post { background:var(--blue); color:#fff; }
.badge.put { background:var(--orange); color:#000; }
.badge.delete { background:var(--red); color:#fff; }
.badge.patch { background:var(--orange); color:#000; }
.badge.ws { background:linear-gradient(135deg, #f093fb 0%, #f5576c 100%); color:#fff; }
.badge.sse { background:linear-gradient(135deg, #43e97b 0%, #38f9d7 100%); color:#000; }
/* 镂空样式（子项） */
.badge.call-outline, .badge.long-outline, .badge.ws-outline, .badge.sse-outline { background:transparent; border:1px solid; }
.badge.call-outline { color:var(--purple); border-color:var(--purple); }
.badge.long-outline { color:#f093fb; border-color:#f093fb; }
.badge.ws-outline { color:#f093fb; border-color:#f093fb; }
.badge.sse-outline { color:#43e97b; border-color:#43e97b; }
.mono { font-family:'SF Mono',Monaco,Consolas,monospace; font-size:15px; }
.text2 { color:var(--text2); }
.text3 { color:var(--text3); }

/* Pagination */
.pagination { display:flex; align-items:center; justify-content:space-between; padding:12px 16px; }
.pagination .info { font-size:16px; color:var(--text2); }
.pagination .buttons { display:flex; gap:6px; }
.pagination button { background:var(--surface); border:1px solid var(--border); border-radius:4px; color:var(--text); padding:6px 12px; cursor:pointer; font-size:15px; }
.pagination button:hover { border-color:var(--blue); }
.pagination button:disabled { opacity:0.4; cursor:default; }

/* Modal Detail View */
.modal-overlay { display:none; position:fixed; top:0; left:0; right:0; bottom:0; background:var(--overlay); z-index:1000; justify-content:center; align-items:flex-start; padding:40px 20px; overflow-y:auto; }
.modal-overlay.active { display:flex; }
.modal-content { background:var(--bg); border:1px solid var(--border); border-radius:12px; width:95vw; max-width:1600px; height:85vh; padding:24px; box-shadow:0 8px 32px var(--shadow); position:relative; display:flex; flex-direction:column; overflow:hidden; }
.modal-close { position:absolute; top:16px; right:16px; background:none; border:none; color:var(--text2); font-size:24px; cursor:pointer; padding:4px 8px; border-radius:4px; }
.modal-close:hover { background:var(--hover); color:var(--text); }
.modal-close-btn { background:var(--surface); border:1px solid var(--border); color:var(--text2); font-size:12px; cursor:pointer; padding:4px 10px; border-radius:4px; }
.modal-close-btn:hover { background:var(--hover); color:var(--text); border-color:var(--red); }

.detail-header { margin-bottom:16px; }
.detail-header h2 { font-size:20px; margin-bottom:12px; }
.detail-header .meta { display:flex; gap:20px; flex-wrap:wrap; }
.detail-header .meta-item { font-size:16px; }
.detail-header .meta-item .label { color:var(--text2); margin-right:4px; }


/* Child List (长连接子项列表) */
.child-list { display:flex; flex-direction:column; }
.child-item {
    display:flex; align-items:center; justify-content:space-between;
    padding:10px 14px; border-bottom:1px solid var(--border); cursor:pointer;
    transition:background 0.15s; gap:12px;
}
.child-item:hover { background:var(--hover); }
.child-item:last-child { border-bottom:none; }
.child-item .left { display:flex; align-items:center; gap:10px; min-width:0; flex:1; }
.child-item .name { font-weight:500; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.child-item .desc { font-size:12px; color:var(--text3); overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.child-item .right { display:flex; align-items:center; gap:12px; flex-shrink:0; }
.child-item .dur { font-family:'SF Mono',Monaco,Consolas,monospace; font-size:13px; }
.load-more-btn {
    display:block; width:100%; padding:12px; text-align:center;
    background:var(--surface2); border:none; color:var(--blue);
    cursor:pointer; font-size:14px; border-top:1px solid var(--border);
}
.load-more-btn:hover { background:var(--hover); }
.tooltip { position:fixed; z-index:9999; background:var(--surface); border:1px solid var(--border); border-radius:8px; padding:10px 14px; font-size:13px; box-shadow:0 4px 12px var(--shadow); pointer-events:none; display:none; max-width:320px; }
.tooltip .tt-name { font-weight:600; margin-bottom:6px; font-size:14px; }
.tooltip .tt-row { display:flex; justify-content:space-between; gap:16px; padding:2px 0; }
.tooltip .tt-label { color:var(--text2); }
.tooltip .tt-val { font-family:'SF Mono',Monaco,Consolas,monospace; }
/* Nested Treemap */
.treemap-scroll { overflow-x:auto; scrollbar-width:thin; scrollbar-color:var(--text3) transparent; }
.treemap-scroll::-webkit-scrollbar { width:6px; height:6px; }
.treemap-scroll::-webkit-scrollbar-track { background:transparent; }
.treemap-scroll::-webkit-scrollbar-thumb { background:var(--text3); border-radius:3px; }
.treemap-scroll::-webkit-scrollbar-thumb:hover { background:var(--text2); }
.treemap-row { display:inline-flex; gap:4px; padding:4px; }
.treemap-cell {
    border:1px solid var(--border); border-radius:6px; cursor:pointer;
    transition:filter 0.15s, box-shadow 0.15s; display:flex; flex-direction:column;
}
.treemap-cell:hover { filter:brightness(1.1); box-shadow:0 2px 8px var(--shadow); }
.treemap-cell.ok { border-left:3px solid var(--blue); background:rgba(88,166,255,0.05); }
.treemap-cell.error { border-left:3px solid var(--red); background:rgba(248,81,73,0.05); }
.treemap-cell.root { border-left:3px solid var(--green); background:rgba(63,185,80,0.04); }
.treemap-label {
    display:flex; align-items:center; justify-content:space-between;
    padding:6px 10px; font-size:13px; white-space:nowrap; overflow:hidden;
    background:rgba(0,0,0,0.06); border-bottom:1px solid var(--border); gap:8px;
}
.treemap-label .name { font-weight:600; overflow:hidden; text-overflow:ellipsis; }
.treemap-label .dur { font-size:12px; opacity:0.7; flex-shrink:0; }
.treemap-idle {
    display:flex; align-items:center; justify-content:center;
    background:repeating-linear-gradient(135deg, var(--surface2), var(--surface2) 4px, var(--border) 4px, var(--border) 5px);
    border:1px dashed var(--border); border-radius:4px;
    font-size:11px; color:var(--text3); flex-shrink:0;
}

/* Span Tree */
.span-tree { background:var(--surface); border:1px solid var(--border); border-radius:8px; overflow:hidden; }
.span-tree-item {
    display:flex; align-items:center; gap:10px;
    padding:6px 12px; border-bottom:1px solid var(--border);
    font-size:13px; transition:background 0.1s;
}
.span-tree-item:last-child { border-bottom:none; }
.span-tree-item:hover { background:var(--hover); }
.span-tree-item .st-name { font-weight:500; min-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.span-tree-item .st-dur { font-family:'SF Mono',Monaco,Consolas,monospace; font-size:12px; color:var(--text2); flex-shrink:0; }
.span-tree-item .st-time { font-size:12px; color:var(--text3); flex-shrink:0; }
.span-tree-item .st-badge { flex-shrink:0; }

/* Loading */
.loading-overlay { display:none; position:absolute; top:0; left:0; right:0; bottom:0; background:color-mix(in srgb, var(--bg) 70%, transparent); backdrop-filter:blur(2px); z-index:10; justify-content:center; align-items:center; border-radius:8px; }
.loading-overlay.active { display:flex; }
.loading-content { display:flex; align-items:center; gap:10px; color:var(--text2); font-size:17px; }
.spinner { width:24px; height:24px; border:3px solid var(--spinner-bg); border-top-color:var(--blue); border-radius:50%; animation:spin 0.6s linear infinite; }
@keyframes spin { to { transform:rotate(360deg); } }
.empty { text-align:center; padding:60px 20px; color:var(--text2); }
.empty-icon { font-size:64px; margin-bottom:12px; opacity:0.3; }
.table-wrap, .stats { position:relative; }
</style>
</head>
<body>
<div class="container">
    <header>
        <h1>⚡ <span>Tracing</span> - 链路追踪</h1>
        <div class="header-right">
            <div class="text2 mono" id="service-name"></div>
            <button class="realtime-toggle" id="realtime-toggle" title="实时推送">
                <span class="realtime-dot"></span> 实时
                <span class="realtime-count" id="realtime-count">0</span>
            </button>
            <button class="theme-toggle" id="theme-toggle" title="切换主题"><span id="theme-icon">🌙</span></button>
        </div>
    </header>
    <div class="stats" id="stats">
        <div class="loading-overlay active" id="stats-loading"><div class="loading-content"><div class="spinner"></div>加载中...</div></div>
    </div>
    <div class="filters">
        <select id="f-status"><option value="">全部状态</option><option value="ok">成功</option><option value="error">错误</option></select>
        <select id="f-transport"><option value="">全部协议</option><option value="call">Call</option><option value="ws">WS</option><option value="long">Long</option><option value="sse">SSE</option></select>
        <input type="text" id="f-handler" placeholder="Handler 名称" style="width:160px">
        <input type="number" id="f-min-dur" placeholder="最小耗时(ms)" style="width:120px">
        <button onclick="doSearch()">查询</button>
        <button class="secondary" onclick="resetFilters()">重置</button>
    </div>
    <div class="table-wrap">
        <table>
            <thead><tr><th>Trace ID</th><th>Handler</th><th>Service</th><th>Transport</th><th>Spans</th><th>耗时</th><th>状态</th><th>时间</th><th>父级</th></tr></thead>
            <tbody id="trace-list"><tr><td colspan="9"><div class="loading-overlay active" style="position:relative;min-height:120px"><div class="loading-content"><div class="spinner"></div>加载中...</div></div></td></tr></tbody>
        </table>
        <div class="pagination" id="pagination"></div>
    </div>
</div>

<!-- Tooltip -->
<div class="tooltip" id="tooltip"></div>

<!-- Modal Detail -->
<div class="modal-overlay" id="modal-overlay">
    <div class="modal-content" id="modal-content">
        <button class="modal-close" id="modal-close">&times;</button>
        <div id="modal-body">
            <div id="detail-header"></div>
            <div class="treemap" id="treemap"></div>
        </div>
    </div>
</div>
<script type="module">
import { TracingClient } from '/code/_tracing/js';

// ═══════════════════════════════════════════════════════════
//  Theme
// ═══════════════════════════════════════════════════════════
function initTheme() { setTheme(localStorage.getItem('tracing-theme') || 'dark'); }
function setTheme(t) { document.documentElement.setAttribute('data-theme', t); localStorage.setItem('tracing-theme', t); document.getElementById('theme-icon').textContent = t === 'dark' ? '🌙' : '☀️'; }
function toggleTheme() { setTheme(document.documentElement.getAttribute('data-theme') === 'dark' ? 'light' : 'dark'); }

// ═══════════════════════════════════════════════════════════
//  State
// ═══════════════════════════════════════════════════════════
let api = null;
let currentPage = 1;
const pageSize = 20;
const MAX_CACHE = 100;
let realtimeEnabled = false;
let evtSource = null;
let pendingCount = 0;       // 未显示的新 trace 数量（累加，不重置）
let cachedTraces = [];       // 实时缓存的 trace 摘要（最多 MAX_CACHE）
let refreshTimer = null;     // 实时模式定时刷新

// ═══════════════════════════════════════════════════════════
//  SSE 实时推送
// ═══════════════════════════════════════════════════════════
function toggleRealtime() {
    realtimeEnabled = !realtimeEnabled;
    const btn = document.getElementById('realtime-toggle');
    btn.classList.toggle('active', realtimeEnabled);
    localStorage.setItem('tracing-realtime', realtimeEnabled ? '1' : '0');
    // 切换时清零计数并刷新列表
    pendingCount = 0;
    updatePendingBadge();
    loadTraces(1);
    // 实时模式下定时刷新（用 API 完整数据覆盖 SSE 预览）
    if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = null; }
    if (realtimeEnabled) {
        refreshTimer = setInterval(() => { loadTraces(currentPage); }, 5000);
    }
}

function connectSSE() {
    if (evtSource) return;
    evtSource = new EventSource('__SSE_PATH__');
    evtSource.addEventListener('trace', (e) => {
        const event = JSON.parse(e.data);
        if (realtimeEnabled) {
            // SSE 推送的已是完整数据，直接插入
            prependTraceRow(event);
            loadStats();
        } else {
            pendingCount++;
            updatePendingBadge();
        }
    });
    evtSource.addEventListener('connected', () => console.log('SSE connected'));
    evtSource.onerror = () => { disconnectSSE(); setTimeout(connectSSE, 3000); };
}

function disconnectSSE() { if (evtSource) { evtSource.close(); evtSource = null; } if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = null; } }

function updatePendingBadge() {
    const el = document.getElementById('realtime-count');
    if (pendingCount > 0) {
        el.textContent = pendingCount > 999 ? '999+' : pendingCount;
        el.classList.add('show');
    } else {
        el.classList.remove('show');
    }
}

function prependTraceRow(t) {
    const tbody = document.getElementById('trace-list');
    // 移除空状态行
    const emptyRow = tbody.querySelector('.empty');
    if (emptyRow) emptyRow.closest('tr')?.remove();
    // 移除已存在的同一 span 行（避免重复）
    const existing = tbody.querySelector(`tr[data-span-id="${t.span_id}"]`);
    if (existing) existing.remove();
    // 插入新行
    const isChild = t.parent_span_id && t.parent_span_id !== '';
    const ti = transportInfo(t, isChild);
    const handler = t.root_handler || t.handler_name || '-';
    const dur = t.duration_us != null && t.duration_us >= 0 ? formatDuration(t.duration_us) : '-';
    const spans = t.span_count > 0 ? t.span_count : '-';
    const nameHtml = isChild
        ? `${esc(handler)} <span style="color:var(--text3);font-size:11px">🔗</span>`
        : esc(handler);
    const parentHtml = isChild
        ? `<span class="mono text3" style="font-size:11px">${t.parent_span_id.substring(0,8)}...</span>`
        : '-';

    const tr = document.createElement('tr');
    tr.className = 'clickable';
    tr.dataset.traceId = t.trace_id;
    tr.dataset.spanId = t.span_id || '';
    tr.dataset.startTime = t.start_time || 0;
    tr.innerHTML = `<td><span class="mono">${t.trace_id.substring(0,16)}...</span></td><td>${nameHtml}</td><td class="text2">${esc(t.service_name)}</td><td><span class="badge ${ti.cls}">${ti.label}</span></td><td class="text2">${spans}</td><td class="mono">${dur}</td><td><span class="badge ${t.has_error ? 'error' : 'ok'}">${t.has_error ? '错误' : '成功'}</span></td><td class="text2">${formatTime(t.start_time)}</td><td>${parentHtml}</td>`;

    // 按 start_time 降序插入（新的在前）
    const startTime = t.start_time || 0;
    let inserted = false;
    for (const row of tbody.querySelectorAll('tr[data-start-time]')) {
        if (parseInt(row.dataset.startTime) < startTime) {
            tbody.insertBefore(tr, row);
            inserted = true;
            break;
        }
    }
    if (!inserted) tbody.appendChild(tr);
    // 限制表格行数
    while (tbody.children.length > MAX_CACHE) tbody.removeChild(tbody.lastChild);
}

// ═══════════════════════════════════════════════════════════
//  Init
// ═══════════════════════════════════════════════════════════
window.doSearch = doSearch;
window.resetFilters = resetFilters;
window.loadTraces = loadTraces;
window.loadMore = loadMore;
window.showDetail = showDetail;
window.closeTopModal = closeTopModal;
window.closeAllModals = closeAllModals;
window.loadChildrenList = loadChildrenList;

window.addEventListener('DOMContentLoaded', async () => {
    initTheme();
    document.getElementById('theme-toggle').addEventListener('click', toggleTheme);
    document.getElementById('realtime-toggle').addEventListener('click', toggleRealtime);

    // Modal 关闭
    document.getElementById('modal-close').addEventListener('click', closeModal);
    document.getElementById('modal-overlay').addEventListener('click', (e) => { if (e.target === e.currentTarget) closeModal(); });
    document.addEventListener('keydown', (e) => { if (e.key === 'Escape') closeModal(); });

    // 表格点击 → 打开 modal
    document.getElementById('trace-list').addEventListener('click', (e) => {
        const row = e.target.closest('tr.clickable');
        if (row) showDetail(row.dataset.traceId, row.dataset.spanId);
    });

    // Tooltip 事件委托
    document.addEventListener('mouseover', (e) => {
        const idle = e.target.closest('.treemap-idle[data-idle-dur]');
        if (idle) { showIdleTooltip(idle, e); return; }
        const cell = e.target.closest('.treemap-cell[data-span-id]');
        if (cell) showTooltip(cell, e);
    });
    document.addEventListener('mousemove', (e) => {
        const tip = document.getElementById('tooltip');
        if (tip.style.display === 'block') positionTooltip(tip, e);
    });
    document.addEventListener('mouseout', (e) => {
        if (e.target.closest('.treemap-idle[data-idle-dur]') || e.target.closest('.treemap-cell[data-span-id]')) hideTooltip();
    });

    if (typeof TracingClient !== 'undefined') {
        const client = new TracingClient({ host: location.hostname, port: parseInt(location.port) || (location.protocol === 'https:' ? 443 : 80), tls: location.protocol === 'https:' });
        api = client.apis;
    }
    // 恢复实时模式状态
    if (localStorage.getItem('tracing-realtime') === '1') {
        realtimeEnabled = true;
        document.getElementById('realtime-toggle').classList.add('active');
    }
    // 始终连接 SSE
    connectSSE();
    loadStats();
    loadTraces();
});

// ═══════════════════════════════════════════════════════════
//  Stats
// ═══════════════════════════════════════════════════════════
async function loadStats() {
    if (!api) return;
    try {
        const resp = await api.stats({ filter: { start_time_from: null, start_time_to: null, service_name: null, transport: null } });
        const s = resp.data;
        document.getElementById('stats').innerHTML = `
            <div class="stat-card"><div class="label">总请求数</div><div class="value">${s.total_traces}</div></div>
            <div class="stat-card"><div class="label">错误数</div><div class="value err">${s.error_traces}</div></div>
            <div class="stat-card"><div class="label">错误率</div><div class="value ${s.error_rate > 5 ? 'err' : 'ok'}">${s.error_rate}%</div></div>
            <div class="stat-card"><div class="label">平均耗时</div><div class="value">${formatDuration(Math.round(s.avg_duration_us))}</div></div>
            <div class="stat-card"><div class="label">P50</div><div class="value">${formatDuration(s.p50_duration_us)}</div></div>
            <div class="stat-card"><div class="label">P99</div><div class="value">${formatDuration(s.p99_duration_us)}</div></div>`;
    } catch(e) { console.error('loadStats error:', e); }
}

// ═══════════════════════════════════════════════════════════
//  Traces List
// ═══════════════════════════════════════════════════════════
async function loadTraces(page) {
    if (!api) { document.getElementById('trace-list').innerHTML = '<tr><td colspan="9"><div class="empty"><div class="empty-icon">📡</div>等待 JS 客户端加载...<br><small>请确保启用了 afast-code 和 afast-js feature</small></div></td></tr>'; return; }
    currentPage = page || 1;
    // 清空待显示计数
    pendingCount = 0;
    updatePendingBadge();

    const tbody = document.getElementById('trace-list');
    tbody.innerHTML = '<tr><td colspan="9"><div class="loading-overlay active" style="position:relative;min-height:120px"><div class="loading-content"><div class="spinner"></div>加载中...</div></div></td></tr>';
    const filter = buildFilter();
    try {
        const resp = await api.list_traces({ page: currentPage, page_size: pageSize, filter });
        const { total, data } = resp.data;
        if (!data || data.length === 0) { tbody.innerHTML = '<tr><td colspan="9"><div class="empty"><div class="empty-icon">📭</div>暂无追踪数据</div></td></tr>'; document.getElementById('pagination').innerHTML = ''; return; }
        tbody.innerHTML = data.map(t => {
            const isChild = t.parent_span_id && t.parent_span_id !== '';
            const ti = transportInfo(t, isChild);
            const nameHtml = isChild
                ? `${esc(t.root_handler)} <span style="color:var(--text3);font-size:11px">🔗</span>`
                : esc(t.parent_span_id ? esc(t.root_handler) : esc(t.root_handler));
            const parentHtml = isChild
                ? `<span class="mono text3" style="font-size:11px">${t.parent_span_id.substring(0,8)}...</span>`
                : '-';
            return `<tr class="clickable" data-trace-id="${t.trace_id}" data-span-id="${t.span_id || ''}"><td><span class="mono">${t.trace_id.substring(0,16)}...</span></td><td>${nameHtml}</td><td class="text2">${esc(t.service_name)}</td><td><span class="badge ${ti.cls}">${ti.label}</span></td><td class="text2">${t.span_count}</td><td class="mono">${formatDuration(t.duration_us)}</td><td><span class="badge ${t.has_error ? 'error' : 'ok'}">${t.has_error ? '错误' : '成功'}</span></td><td class="text2">${formatTime(t.start_time)}</td><td>${parentHtml}</td></tr>`;
        }).join('');
        const totalPages = Math.ceil(total / pageSize);
        document.getElementById('pagination').innerHTML = `<div class="info">共 ${total} 条，第 ${currentPage}/${totalPages} 页</div><div class="buttons"><button ${currentPage <= 1 ? 'disabled' : ''} onclick="loadTraces(${currentPage-1})">上一页</button><button ${currentPage >= totalPages ? 'disabled' : ''} onclick="loadTraces(${currentPage+1})">下一页</button></div>`;
    } catch(e) { console.error('loadTraces error:', e); tbody.innerHTML = `<tr><td colspan="9"><div class="empty">加载失败: ${esc(e.message)}</div></td></tr>`; }
}

function buildFilter() {
    const f = { start_time_from: null, start_time_to: null, service_name: null, transport: null, status: null, min_duration_us: null, max_duration_us: null, error_code: null, handler_name: null };
    const status = document.getElementById('f-status').value;
    const transport = document.getElementById('f-transport').value;
    const handler = document.getElementById('f-handler').value.trim();
    const minDur = document.getElementById('f-min-dur').value;
    if (status) f.status = { tag: status === 'error' ? 'Error' : 'Ok' };
    if (transport) {
        // 映射 badge 标签到 transport 值
        const transportMap = { call: ['http', 'http-binary'], long: ['ws-binary', 'tcp'] };
        if (transportMap[transport]) {
            f.transport = transportMap[transport];
        } else {
            f.transport = transport;
        }
    }
    if (handler) f.handler_name = handler;
    if (minDur) f.min_duration_us = parseInt(minDur) * 1000; // ms → μs
    return f;
}

function doSearch() { currentPage = 1; loadTraces(1); }
function resetFilters() { document.getElementById('f-status').value = ''; document.getElementById('f-transport').value = ''; document.getElementById('f-handler').value = ''; document.getElementById('f-min-dur').value = ''; doSearch(); }

// ═══════════════════════════════════════════════════════════
//  Detail Modal
// ═══════════════════════════════════════════════════════════
let modalStack = [];

async function showDetail(traceId, spanId) {
    if (!api) return;
    const zIndex = 1000 + modalStack.length;
    const modal = document.createElement('div');
    modal.className = 'modal-overlay active';
    modal.style.zIndex = zIndex;
    modal.innerHTML = `<div class="modal-content">
        <div style="display:flex;gap:8px;position:absolute;top:16px;right:16px;z-index:1">
            <button class="modal-close-btn" onclick="closeAllModals()" title="关闭所有">✕ 全部</button>
            <button class="modal-close-btn" onclick="closeTopModal()" title="关闭当前">✕</button>
        </div>
        <div class="modal-body">
            <div class="detail-header" id="dh-${zIndex}"><div style="padding:40px;text-align:center"><div class="spinner" style="margin:0 auto"></div></div></div>
            <div class="treemap-scroll"><div class="treemap" id="tm-${zIndex}"></div></div>
        </div>
    </div>`;
    document.body.appendChild(modal);
    modalStack.push(modal);
    modal.addEventListener('click', (e) => { if (e.target === modal) closeTopModal(); });

    try {
        const resp = await api.get_trace({ trace_id: traceId });
        const spans = resp.spans || [];
        console.log('[get_trace] resp:', JSON.parse(JSON.stringify(resp)));
        console.log('[get_trace] spans:', JSON.parse(JSON.stringify(spans)));
        if (spans.length === 0) { document.getElementById(`dh-${zIndex}`).innerHTML = '<div class="empty">无数据</div>'; return; }

        const root = spanId
            ? spans.find(s => s.span_id === spanId) || spans[0]
            : spans.find(s => !s.parent_span_id) || spans[0];
        if (!root) { document.getElementById(`dh-${zIndex}`).innerHTML = '<div class="empty">无数据</div>'; return; }

        const rti = transportInfo(root);
        const statusCls = root.status?.tag === 'Error' ? 'error' : 'ok';
        const statusLabel = root.status?.tag === 'Error' ? '错误' : '成功';
        document.getElementById(`dh-${zIndex}`).innerHTML = `
            <h2 style="margin-bottom:12px">${esc(root.handler_name)} <span class="mono text2" style="font-size:13px">${root.span_id}</span></h2>
            <div style="display:flex;gap:20px;flex-wrap:wrap;margin-bottom:16px;font-size:14px">
                <span><b>Trace:</b> <span class="mono">${traceId.substring(0,16)}...</span></span>
                <span><b>Service:</b> ${esc(root.service_name)}</span>
                <span><b>Transport:</b> <span class="badge ${rti.cls}">${rti.label}</span></span>
                <span><b>耗时:</b> ${formatDuration(root.duration_us)}</span>
                <span><b>状态:</b> <span class="badge ${statusCls}">${statusLabel}</span></span>
            </div>`;

        const isLongConn = root.long_connection && !root.parent_span_id;
        const tm = document.getElementById(`tm-${zIndex}`);

        if (isLongConn) {
            // 长连接：列表视图
            tm.innerHTML = `<div class="child-list" id="cl-${zIndex}" data-trace-id="${traceId}" data-span-id="${root.span_id}" data-loaded="0"></div>`;
            await loadChildrenList(traceId, root.span_id, zIndex, 0);
        } else {
            // 普通请求：嵌套树图 + 树形列表
            tm.innerHTML = `<div class="treemap-scroll" id="treemap-${zIndex}"></div><div style="margin-top:16px">${renderSpanTree(spans, traceId)}</div>`;
            renderTreemapDOM(`treemap-${zIndex}`, spans, traceId, root);
        }
    } catch(e) {
        console.error('showDetail error:', e);
        document.getElementById(`dh-${zIndex}`).innerHTML = `<div class="empty">加载失败: ${esc(e.message)}</div>`;
    }
}

function closeTopModal() {
    const modal = modalStack.pop();
    if (modal) modal.remove();
}


function closeAllModals() {
    while (modalStack.length) {
        const m = modalStack.pop();
        if (m) m.remove();
    }
}

const PAGE_SIZE = 20;

// ═══════════════════════════════════════════════════════════
//  Span Tree (层级列表)
// ═══════════════════════════════════════════════════════════
function renderSpanTree(spans, traceId) {
    const byId = {};
    spans.forEach(s => byId[s.span_id] = s);
    const childrenMap = {};
    spans.forEach(s => {
        const pid = s.parent_span_id || '__root__';
        (childrenMap[pid] = childrenMap[pid] || []).push(s);
    });
    Object.values(childrenMap).forEach(arr => arr.sort((a, b) => a.start_time - b.start_time));
    const roots = spans.filter(s => !s.parent_span_id || !byId[s.parent_span_id]).sort((a, b) => a.start_time - b.start_time);

    function renderLevel(spanId, depth) {
        const span = byId[spanId];
        if (!span) return '';
        const children = childrenMap[spanId] || [];
        const name = span.handler_name || '(unknown)';
        const dur = formatDuration(span.duration_us);
        const time = formatDateTime(span.start_time);
        const statusCls = span.status?.tag === 'Error' ? 'error' : 'ok';
        const indent = depth * 20;
        const prefix = depth > 0 ? '└ ' : '';

        let html = `<div class="span-tree-item" style="padding-left:${indent + 12}px">
            <span class="st-badge badge ${statusCls}">${statusCls === 'error' ? '❌' : '✅'}</span>
            <span class="st-name">${prefix}${esc(name)}</span>
            <span class="st-dur">${dur}</span>
            <span class="st-time">${time}</span>
        </div>`;

        for (const child of children) {
            html += renderLevel(child.span_id, depth + 1);
        }
        return html;
    }

    let treeHtml = '';
    for (const root of roots) {
        treeHtml += renderLevel(root.span_id, 0);
    }
    return `<div class="span-tree">${treeHtml}</div>`;
}

// ═══════════════════════════════════════════════════════════
//  Nested Div Treemap
// ═══════════════════════════════════════════════════════════
const CELL_PAD = 4;
const PX_PER_MS = 100;  // 0.1px per μs
const PX_ZERO = 60;     // < 1000μs 固定 60px

function spanPx(us) { return us < 1000 ? PX_ZERO : (us / 1000) * PX_PER_MS; }

// 递归计算每个 span 的渲染宽度（从叶子向上）
// 包含间隙：子级之间的空闲时间也有宽度
function calcTree(spans, parentId) {
    const children = spans.filter(s => (s.parent_span_id || null) === parentId)
        .sort((a, b) => a.start_time - b.start_time);
    if (children.length === 0) return [];

    // 构建带间隙的列表
    const items = [];
    let cursor = children[0].start_time;
    for (const span of children) {
        const gap = span.start_time - cursor;
        if (gap > 0) {
            items.push({ type: 'idle', duration_us: gap, totalW: (gap / 1000) * PX_PER_MS });
        }
        const childNodes = calcTree(spans, span.span_id);
        const selfW = spanPx(span.duration_us);
        if (childNodes.length === 0) {
            items.push({ type: 'span', span, children: childNodes, totalW: selfW });
            cursor = span.start_time + span.duration_us;
            continue;
        }
        const childrenW = childNodes.reduce((s, c) => s + c.totalW, 0)
            + (childNodes.length - 1) * CELL_PAD
            + CELL_PAD * 2 + 4; // +4 for cell border (1px × 2 sides × 2)
        const totalW = Math.max(selfW, childrenW);
        items.push({ type: 'span', span, children: childNodes, totalW });
        cursor = span.start_time + span.duration_us;
    }
    return items;
}

function renderNode(item, traceId) {
    if (item.type === 'idle') {
        return `<div class="treemap-idle" style="width:${item.totalW}px;flex:0 0 ${item.totalW}px" data-idle-dur="${item.duration_us}"></div>`;
    }
    const s = item.span;
    const w = item.totalW;
    const statusCls = s.status?.tag === 'Error' ? 'error' : 'ok';
    const dur = formatDuration(s.duration_us);
    const name = s.handler_name || '(unknown)';
    const desc = s.handler_desc || '';
    const dataAttr = `data-span-id="${s.span_id}" data-name="${esc(name)}" data-desc="${esc(desc)}" data-dur="${dur}" data-status="${statusCls}"`;

    if (item.children.length === 0) {
        return `<div class="treemap-cell ${statusCls}" ${dataAttr} style="width:${w}px;flex:0 0 ${w}px" onclick="event.stopPropagation();showDetail('${traceId}','${s.span_id}')">
            <div class="treemap-label"><span class="name">${esc(name)}</span><span class="dur">${dur}</span></div>
        </div>`;
    }

    const childHtml = item.children.map(c => renderNode(c, traceId)).join('');
    return `<div class="treemap-cell ${statusCls}" ${dataAttr} style="width:${w}px;flex:0 0 ${w}px" onclick="event.stopPropagation();showDetail('${traceId}','${s.span_id}')">
        <div class="treemap-label"><span class="name">${esc(name)}</span><span class="dur">${dur}</span></div>
        <div class="treemap-row">${childHtml}</div>
    </div>`;
}


function renderTreemapDOM(containerId, spans, traceId, root) {
    const container = document.getElementById(containerId);
    const childItems = calcTree(spans, root.span_id);
    const rootSelfW = spanPx(root.duration_us);
    const rootChildrenW = childItems.length > 0
        ? childItems.reduce((s, c) => s + c.totalW, 0)
          + (childItems.length - 1) * CELL_PAD
          + CELL_PAD * 2 + 4
        : 0;
    const rootW = Math.max(rootSelfW, rootChildrenW);
    const rootStatus = root.status?.tag === 'Error' ? 'error' : 'root';
    const rootName = root.handler_name || '(unknown)';
    const rootDur = formatDuration(root.duration_us);

    if (childItems.length === 0) {
        container.innerHTML = `<div class="treemap-row"><div class="treemap-cell ${rootStatus}" style="width:${rootW}px;flex:0 0 ${rootW}px">
            <div class="treemap-label"><span class="name">${esc(rootName)}</span><span class="dur">${rootDur}</span></div>
        </div></div>`;
        return;
    }

    const childHtml = childItems.map(c => renderNode(c, traceId)).join('');
    container.innerHTML = `<div class="treemap-row"><div class="treemap-cell ${rootStatus}" style="width:${rootW}px;flex:0 0 ${rootW}px">
        <div class="treemap-label"><span class="name">${esc(rootName)}</span><span class="dur">${rootDur}</span></div>
        <div class="treemap-row" style="padding:${CELL_PAD}px">${childHtml}</div>
    </div></div>`;
}

// 长连接子项列表
async function loadChildrenList(traceId, parentId, zIndex, loaded) {
    const page = Math.floor(loaded / PAGE_SIZE) + 1;
    try {
        const resp = await api.get_children({ trace_id: traceId, parent_span_id: parentId, page, page_size: PAGE_SIZE });
        const { total, data } = resp.data;
        const list = document.getElementById(`cl-${zIndex}`);
        if (!list || !data || data.length === 0) return;

        const oldBtn = list.querySelector('.load-more-btn');
        if (oldBtn) oldBtn.remove();

        data.forEach(s => {
            const item = document.createElement('div');
            item.className = 'child-item';
            const statusCls = s.status?.tag === 'Error' ? 'error' : 'ok';
            const ti = transportInfo(s);
            item.innerHTML = `
                <div class="left">
                    <span class="badge ${statusCls}" style="flex-shrink:0">${s.status?.tag === 'Error' ? '❌' : '✅'}</span>
                    <span class="name">${esc(s.handler_name)}</span>
                    <span class="desc">${esc(s.handler_desc)}</span>
                </div>
                <div class="right">
                    <span class="badge ${ti.cls}">${ti.label}</span>
                    <span class="dur">${formatDuration(s.duration_us)}</span>
                    <span class="text3" style="font-size:11px">${formatTime(s.start_time)}</span>
                </div>`;
            item.onclick = () => showDetail(traceId, s.span_id);
            list.appendChild(item);
        });

        const newLoaded = loaded + data.length;
        if (newLoaded < total) {
            const btn = document.createElement('button');
            btn.className = 'load-more-btn';
            btn.textContent = `加载更早的数据 (${newLoaded}/${total})`;
            btn.onclick = () => loadChildrenList(traceId, parentId, zIndex, newLoaded);
            list.appendChild(btn);
        }
    } catch(e) {
        console.error('loadChildrenList error:', e);
    }
}

async function loadMore(traceId, parentId, parentDuration, loaded, maxDepth) {
    const page = Math.floor(loaded / PAGE_SIZE) + 1;
    try {
        const resp = await api.get_children({ trace_id: traceId, parent_span_id: parentId, page, page_size: PAGE_SIZE });
        const { total, data } = resp.data;
        const container = document.querySelector(`[data-children-of="${parentId}"]`);
        if (!container || !data || data.length === 0) return;

        // 移除旧的加载按钮
        const oldBtn = container.querySelector('.treemap-load');
        if (oldBtn) oldBtn.remove();

        // 追加新的 treemap 行
        const newRow = document.createElement('div');
        newRow.innerHTML = renderTreemap(traceId, data, parentDuration);
        while (newRow.firstChild) container.appendChild(newRow.firstChild);

        const newLoaded = loaded + data.length;
        if (newLoaded < total) {
            container.innerHTML += `<div class="treemap-load" onclick="loadMore('${traceId}', '${parentId}', ${parentDuration}, ${newLoaded}, ${maxDepth})">+${total - newLoaded} 更多</div>`;
        }
    } catch(e) {
        console.error('loadMore error:', e);
    }
}

function closeModal() { document.getElementById('modal-overlay').classList.remove('active'); }

// ═══════════════════════════════════════════════════════════
//  Utils
// ═══════════════════════════════════════════════════════════
function formatDuration(us) { if (us === 0) return '0μs'; if (us < 1000) return us + 'μs'; if (us < 1000000) return (us/1000).toFixed(1) + 'ms'; if (us < 60000000) return (us/1000000).toFixed(2) + 's'; return (us/60000000).toFixed(1) + 'min'; }
function formatTime(ts) { const d = new Date(ts); const pad = n => String(n).padStart(2,'0'); return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`; }
function formatDateTime(ts) { const d = new Date(ts); const pad = n => String(n).padStart(2,'0'); return `${d.getFullYear()}年${pad(d.getMonth()+1)}月${pad(d.getDate())}日 ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`; }
function esc(s) { if (!s) return ''; const el = document.createElement('span'); el.textContent = s; return el.innerHTML; }

// ═══════════════════════════════════════════════════════════
//  Tooltip
// ═══════════════════════════════════════════════════════════
function showTooltip(cell, e) {
    const tip = document.getElementById('tooltip');
    const name = cell.dataset.name || '';
    const desc = cell.dataset.desc || '-';
    const dur = cell.dataset.dur || '';
    const spanId = cell.dataset.spanId || '';
    const isError = cell.dataset.status === 'error';
    const status = isError ? '❌ 错误' : '✅ 成功';

    tip.innerHTML = `
        <div class="tt-name">${esc(name)}</div>
        <div class="tt-row"><span class="tt-label">描述</span><span>${esc(desc)}</span></div>
        <div class="tt-row"><span class="tt-label">耗时</span><span class="tt-val">${esc(dur)}</span></div>
        <div class="tt-row"><span class="tt-label">状态</span><span>${status}</span></div>
        <div class="tt-row"><span class="tt-label">Span ID</span><span class="tt-val">${esc(spanId)}</span></div>
    `;
    tip.style.display = 'block';
    positionTooltip(tip, e);
}

function positionTooltip(tip, e) {
    const pad = 12;
    let x = e.clientX + pad;
    let y = e.clientY + pad;
    const rect = tip.getBoundingClientRect();
    if (x + rect.width > window.innerWidth) x = e.clientX - rect.width - pad;
    if (y + rect.height > window.innerHeight) y = e.clientY - rect.height - pad;
    tip.style.left = x + 'px';
    tip.style.top = y + 'px';
}

function showIdleTooltip(el, e) {
    const tip = document.getElementById('tooltip');
    const dur = parseInt(el.dataset.idleDur) || 0;
    tip.innerHTML = `
        <div class="tt-name">空闲间隔</div>
        <div class="tt-row"><span class="tt-label">持续</span><span class="tt-val">${formatDuration(dur)}</span></div>
    `;
    tip.style.display = 'block';
    positionTooltip(tip, e);
}

function hideTooltip() {
    document.getElementById('tooltip').style.display = 'none';
}

function transportInfo(t, outline) {
    const suffix = outline ? '-outline' : '';
    if (t.transport === 'sse') return { label: 'SSE', cls: 'sse' + suffix };
    if (t.transport === 'ws' && !t.is_binary) return { label: 'WS', cls: 'ws' + suffix };
    if (t.is_binary) return { label: t.long_connection ? 'Long' : 'Call', cls: (t.long_connection ? 'long' : 'call') + suffix };
    if (outline) return { label: 'Call', cls: 'call-outline' };
    const m = (t.method || '').toUpperCase();
    if (m === 'GET') return { label: 'GET', cls: 'get' };
    if (m === 'POST') return { label: 'POST', cls: 'post' };
    if (m === 'PUT') return { label: 'PUT', cls: 'put' };
    if (m === 'DELETE') return { label: 'DELETE', cls: 'delete' };
    if (m === 'PATCH') return { label: 'PATCH', cls: 'patch' };
    return { label: 'Call', cls: 'call' };
}
</script>
</body>
</html>
"#;
