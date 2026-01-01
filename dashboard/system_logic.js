
// This is a temporary file to hold the new JS logic before injecting it
// Format: bytes to human readable
function formatBytes(bytes, decimals = 2) {
    if (!+bytes) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

async function loadSystemData() {
    const tbody = document.getElementById('system-tbody');
    tbody.innerHTML = '<tr><td colspan="5" class="loading"><div class="spinner"></div> Analyze System Spirituality...</td></tr>';

    // Status placeholders
    document.getElementById('sys-status').innerHTML = '<span class="loading">...</span>';
    document.getElementById('sys-ratelimit').innerHTML = '<span class="loading">...</span>';
    document.getElementById('sys-cache-size').innerHTML = '<span class="loading">...</span>';
    document.getElementById('sys-cache-files').innerHTML = '<span class="loading">...</span>';

    try {
        // Parallel fetch for spiritual speed
        const [healthRes, limitRes, cacheRes] = await Promise.all([
            fetch(`${API_BASE}/health`),
            fetch(`${API_BASE}/rate-limit`),
            fetch(`${API_BASE}/v1/api/kaspa/cache/stats`)
        ]);

        // 1. Health
        const health = await healthRes.json();
        const statusColor = health.status === 'ok' ? 'var(--positive)' : 'var(--negative)';
        document.getElementById('sys-status').innerHTML = `<span style="color:${statusColor}">●</span> ${health.status.toUpperCase()}`;

        // 2. Rate Limits
        const limits = await limitRes.json();
        const core = limits.resources.core;
        document.getElementById('sys-ratelimit').textContent = `${core.remaining} / ${core.limit}`;
        const resetTime = new Date(core.reset * 1000).toLocaleTimeString();
        document.getElementById('sys-ratelimit-reset').textContent = `Resets at ${resetTime}`;

        // 3. Cache Stats
        const cache = await cacheRes.json();
        document.getElementById('sys-cache-size').textContent = formatBytes(cache.total_size_bytes);
        document.getElementById('sys-cache-files').textContent = formatNumber(cache.total_keys);

        // Populate Table
        const categories = Object.entries(cache.categories).sort((a, b) => b[1].size_bytes - a[1].size_bytes);

        tbody.innerHTML = categories.map(([cat, stats]) => `
            <tr>
                <td><span class="ticker-badge" style="background:var(--bg-secondary);border:1px solid var(--accent);color:var(--accent)">${cat}</span></td>
                <td style="color:var(--text-primary)">${stats.description}</td>
                <td>${formatNumber(stats.keys)}</td>
                <td style="font-family:monospace">${formatBytes(stats.size_bytes)}</td>
                <td><span style="color:var(--positive)">Active</span></td>
            </tr>
        `).join('');

    } catch (e) {
        console.error('System load failed:', e);
        tbody.innerHTML = `<tr><td colspan="5" class="empty-state" style="color:var(--negative)">System Meditation Interrupted: ${e.message}</td></tr>`;
    }
}
