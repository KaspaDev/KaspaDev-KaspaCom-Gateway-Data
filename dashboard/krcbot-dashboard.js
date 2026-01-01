
// Auto-detect API base URL
let API_BASE = window.location.origin;

// Handle local file access or frontend dev server
if (window.location.protocol === 'file:') {
    // For file:// protocol, try to detect from common ports
    API_BASE = 'http://localhost:8088';  // Dockerized default (Envoy proxy)
} else if (window.location.port === '3000') {
    // Frontend dev server on port 3000
    API_BASE = 'http://localhost:3010';
} else if (window.location.port === '8088') {
    // Already on port 8088 (dockerized)
    API_BASE = window.location.origin;
} else if (!window.location.port || window.location.port === '80' || window.location.port === '443') {
    // Production or standard ports - use same origin
    API_BASE = window.location.origin;
}

// Remove trailing slash if present
API_BASE = API_BASE.replace(/\/$/, '');

// Log API base for debugging (only in development)
if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
    console.log('Dashboard API Base URL:', API_BASE);
}

let currentTab = 'trends';  // Default to Trends & Stats
let krc20Data = [];
let krc721Data = [];
let knsData = [];

// Formatters - high precision for crypto prices
const formatKAS = (n) => n != null ? n.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 8 }) : '--';
const formatUSD = (n) => n != null ? `$${n.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 12 })}` : '--';
const formatNumber = (n) => n != null ? n.toLocaleString() : '--';
const truncateAddress = (addr) => addr ? `${addr.slice(0, 10)}...${addr.slice(-6)}` : '--';

// Pagination state
const PAGE_SIZE = 25;
let krc20Page = 1;
let krc721Page = 1;
let knsPage = 1;

// Render pagination controls
function renderPagination(containerId, currentPage, totalItems, onPageChange) {
    const container = document.getElementById(containerId);
    const totalPages = Math.ceil(totalItems / PAGE_SIZE);

    if (totalPages <= 1) {
        container.innerHTML = '';
        return;
    }

    let html = `<button class="page-btn" ${currentPage === 1 ? 'disabled' : ''} data-page="${currentPage - 1}">← Prev</button>`;

    // Show page numbers
    const startPage = Math.max(1, currentPage - 2);
    const endPage = Math.min(totalPages, currentPage + 2);

    if (startPage > 1) {
        html += `<button class="page-btn" data-page="1">1</button>`;
        if (startPage > 2) html += `<span class="page-info">...</span>`;
    }

    for (let i = startPage; i <= endPage; i++) {
        html += `<button class="page-btn ${i === currentPage ? 'active' : ''}" data-page="${i}">${i}</button>`;
    }

    if (endPage < totalPages) {
        if (endPage < totalPages - 1) html += `<span class="page-info">...</span>`;
        html += `<button class="page-btn" data-page="${totalPages}">${totalPages}</button>`;
    }

    html += `<span class="page-info">Page ${currentPage} of ${totalPages} (${totalItems} items)</span>`;
    html += `<button class="page-btn" ${currentPage === totalPages ? 'disabled' : ''} data-page="${currentPage + 1}">Next →</button>`;

    container.innerHTML = html;

    // Add click handlers
    container.querySelectorAll('.page-btn:not(:disabled)').forEach(btn => {
        btn.addEventListener('click', () => onPageChange(parseInt(btn.dataset.page)));
    });
}

// Tab Navigation
document.querySelectorAll('.tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
        document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
        btn.classList.add('active');
        document.getElementById(`tab-${btn.dataset.tab}`).classList.add('active');
        currentTab = btn.dataset.tab;
        loadTabData();
    });
});

// Search handlers - reset page on search/sort change
document.getElementById('krc20-search').addEventListener('input', () => filterKRC20(true));
document.getElementById('krc721-search').addEventListener('input', () => filterKRC721(true));
document.getElementById('kns-search').addEventListener('input', () => filterKNS(true));
document.getElementById('krc20-sort').addEventListener('change', () => filterKRC20(true));
document.getElementById('krc721-sort').addEventListener('change', () => filterKRC721(true));
document.getElementById('kns-sort').addEventListener('change', () => filterKNS(true));
document.getElementById('trends-timeframe').addEventListener('change', loadTrendsData);

// Hot mints interval buttons
document.querySelectorAll('.range-btn[data-interval]').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.range-btn[data-interval]').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        loadHotMints(btn.dataset.interval);
    });
});

// Check API status and cache stats
async function checkStatus() {
    try {
        const healthUrl = `${API_BASE}/health`;
        const res = await fetch(healthUrl);
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const data = await res.json();
        document.getElementById('api-status').textContent = data.status === 'ok' ? 'API Online' : 'Degraded';

        // Also fetch cache stats for header
        try {
            const cacheRes = await fetch(`${API_BASE}/v1/api/kaspa/cache/stats`);
            if (cacheRes.ok) {
                const cache = await cacheRes.json();

                // Calculate total requests, hits, misses from all categories
                let totalRequests = 0;
                let totalHits = 0;
                let totalMisses = 0;

                if (cache.categories) {
                    Object.values(cache.categories).forEach(cat => {
                        totalRequests += cat.requests || 0;
                        totalHits += cat.hits || 0;
                        totalMisses += cat.misses || 0;
                    });
                }

                // Use totalHits from categories, fallback to cache_hits
                const displayHits = totalHits > 0 ? totalHits : (cache.cache_hits || 0);
                const hitRate = totalRequests > 0 ? ((totalHits / totalRequests) * 100).toFixed(1) :
                    (cache.cache_hits > 0 ? '100.0' : '0.0');

                document.getElementById('cache-hits-header').textContent = formatNumber(displayHits);
                document.getElementById('cache-hit-rate').textContent = `${hitRate}%`;
            }
        } catch (e) {
            console.warn('Failed to load cache stats for header:', e);
            document.getElementById('cache-hits-header').textContent = '--';
            document.getElementById('cache-hit-rate').textContent = '--';
        }
    } catch (e) {
        console.error('API health check failed:', e);
        document.getElementById('api-status').textContent = 'Offline';
        document.getElementById('cache-hits-header').textContent = '--';
        document.getElementById('cache-hit-rate').textContent = '--';
    }
}

// Load KRC20 Data
async function loadKRC20Data() {
    const tbody = document.getElementById('krc20-tbody');
    tbody.innerHTML = '<tr><td colspan="3" class="loading"><div class="spinner"></div> Loading...</td></tr>';

    try {
        const res = await fetch(`${API_BASE}/v1/api/kaspa/floor-price`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        krc20Data = await res.json();

        document.getElementById('krc20-total').textContent = krc20Data.length;
        document.getElementById('krc20-count').textContent = `${krc20Data.length} tokens`;

        if (krc20Data.length > 0) {
            const sorted = [...krc20Data].sort((a, b) => (b.floor_price || 0) - (a.floor_price || 0));
            document.getElementById('krc20-top').textContent = sorted[0]?.ticker || '--';
        }

        filterKRC20();

        // Load trade stats
        const statsRes = await fetch(`${API_BASE}/v1/api/kaspa/trade-stats?timeFrame=24h`);
        if (statsRes.ok) {
            const stats = await statsRes.json();
            document.getElementById('krc20-volume').textContent = stats.totalVolumeKasKaspiano || '--';
            document.getElementById('krc20-trades').textContent = formatNumber(stats.totalTradesKaspiano);
        }
    } catch (e) {
        console.error('KRC20 load failed:', e);
        tbody.innerHTML = `<tr><td colspan="3" class="empty-state">Failed to load: ${e.message}</td></tr>`;
    }
}

function filterKRC20(resetPage = false) {
    if (resetPage) krc20Page = 1;

    const search = document.getElementById('krc20-search').value.toLowerCase();
    const sort = document.getElementById('krc20-sort').value;
    const tbody = document.getElementById('krc20-tbody');

    let filtered = krc20Data.filter(t => t.ticker.toLowerCase().includes(search));

    switch (sort) {
        case 'price-desc': filtered.sort((a, b) => (b.floor_price || 0) - (a.floor_price || 0)); break;
        case 'price-asc': filtered.sort((a, b) => (a.floor_price || 0) - (b.floor_price || 0)); break;
        case 'ticker-asc': filtered.sort((a, b) => a.ticker.localeCompare(b.ticker)); break;
    }

    if (filtered.length === 0) {
        tbody.innerHTML = '<tr><td colspan="3" class="empty-state"><div class="icon">🔍</div>No tokens found</td></tr>';
        document.getElementById('krc20-pagination').innerHTML = '';
        return;
    }

    // Paginate
    const start = (krc20Page - 1) * PAGE_SIZE;
    const pageData = filtered.slice(start, start + PAGE_SIZE);

    tbody.innerHTML = pageData.map(t => `
                <tr onclick="showTokenDetail('${t.ticker}')">
                    <td><span class="ticker-badge">${t.ticker}</span></td>
                    <td>${formatKAS(t.floor_price)}</td>
                    <td>${formatUSD(t.floor_price * 0.043)}</td>
                </tr>
            `).join('');

    renderPagination('krc20-pagination', krc20Page, filtered.length, (page) => {
        krc20Page = page;
        filterKRC20();
    });
}

// Load KRC721 Data
async function loadKRC721Data() {
    const tbody = document.getElementById('krc721-tbody');
    tbody.innerHTML = '<tr><td colspan="2" class="loading"><div class="spinner"></div> Loading...</td></tr>';

    try {
        const res = await fetch(`${API_BASE}/v1/api/kaspa/krc721/floor-price`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        krc721Data = await res.json();

        document.getElementById('krc721-collections').textContent = krc721Data.length;
        document.getElementById('krc721-count').textContent = `${krc721Data.length} collections`;

        if (krc721Data.length > 0) {
            const sorted = [...krc721Data].sort((a, b) => (b.floor_price || 0) - (a.floor_price || 0));
            document.getElementById('krc721-hot').textContent = sorted[0]?.ticker || '--';
        }

        filterKRC721();

        // Load trade stats
        const statsRes = await fetch(`${API_BASE}/v1/api/kaspa/krc721/trade-stats?timeFrame=24h`);
        if (statsRes.ok) {
            const stats = await statsRes.json();
            document.getElementById('krc721-volume').textContent = stats.totalVolumeKasKaspiano || '--';
            document.getElementById('krc721-sales').textContent = formatNumber(stats.totalTradesKaspiano);
        }
    } catch (e) {
        console.error('KRC721 load failed:', e);
        tbody.innerHTML = `<tr><td colspan="2" class="empty-state">Failed to load: ${e.message}</td></tr>`;
    }
}

function filterKRC721(resetPage = false) {
    if (resetPage) krc721Page = 1;

    const search = document.getElementById('krc721-search').value.toLowerCase();
    const sort = document.getElementById('krc721-sort').value;
    const tbody = document.getElementById('krc721-tbody');

    let filtered = krc721Data.filter(t => t.ticker.toLowerCase().includes(search));

    switch (sort) {
        case 'floor-desc': filtered.sort((a, b) => (b.floor_price || 0) - (a.floor_price || 0)); break;
        case 'floor-asc': filtered.sort((a, b) => (a.floor_price || 0) - (b.floor_price || 0)); break;
        case 'ticker-asc': filtered.sort((a, b) => a.ticker.localeCompare(b.ticker)); break;
    }

    if (filtered.length === 0) {
        tbody.innerHTML = '<tr><td colspan="2" class="empty-state"><div class="icon">🎨</div>No collections found</td></tr>';
        document.getElementById('krc721-pagination').innerHTML = '';
        return;
    }

    // Paginate
    const start = (krc721Page - 1) * PAGE_SIZE;
    const pageData = filtered.slice(start, start + PAGE_SIZE);

    tbody.innerHTML = pageData.map(t => `
                <tr onclick="showCollectionDetail('${t.ticker}')">
                    <td><span class="ticker-badge" style="background:var(--krc721-color)">${t.ticker}</span></td>
                    <td>${formatKAS(t.floor_price)} KAS</td>
                </tr>
            `).join('');

    renderPagination('krc721-pagination', krc721Page, filtered.length, (page) => {
        krc721Page = page;
        filterKRC721();
    });
}

// Load KNS Data
async function loadKNSData() {
    const tbody = document.getElementById('kns-tbody');
    tbody.innerHTML = '<tr><td colspan="3" class="loading"><div class="spinner"></div> Loading...</td></tr>';

    try {
        const res = await fetch(`${API_BASE}/v1/api/kaspa/kns/listed-orders`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        knsData = await res.json();

        document.getElementById('kns-listed').textContent = knsData.length;
        document.getElementById('kns-count').textContent = `${knsData.length} domains`;

        if (knsData.length > 0) {
            const avgPrice = knsData.reduce((sum, d) => sum + (d.price || 0), 0) / knsData.length;
            document.getElementById('kns-avg').textContent = formatKAS(avgPrice) + ' KAS';
        }

        filterKNS();

        // Load trade stats
        const statsRes = await fetch(`${API_BASE}/v1/api/kaspa/kns/trade-stats?timeFrame=24h`);
        if (statsRes.ok) {
            const stats = await statsRes.json();
            document.getElementById('kns-volume').textContent = stats.totalVolumeKasKaspiano || '--';
            document.getElementById('kns-sales').textContent = formatNumber(stats.totalTradesKaspiano);
        }
    } catch (e) {
        console.error('KNS load failed:', e);
        tbody.innerHTML = `<tr><td colspan="3" class="empty-state">Failed to load: ${e.message}</td></tr>`;
    }
}

function filterKNS(resetPage = false) {
    if (resetPage) knsPage = 1;

    const search = document.getElementById('kns-search').value.toLowerCase();
    const sort = document.getElementById('kns-sort').value;
    const tbody = document.getElementById('kns-tbody');

    let filtered = knsData.filter(d => (d.assetId || d.asset_id || '').toLowerCase().includes(search));

    switch (sort) {
        case 'price-desc': filtered.sort((a, b) => (b.price || 0) - (a.price || 0)); break;
        case 'price-asc': filtered.sort((a, b) => (a.price || 0) - (b.price || 0)); break;
        case 'name-asc': filtered.sort((a, b) => (a.assetId || a.asset_id || '').localeCompare(b.assetId || b.asset_id || '')); break;
    }

    if (filtered.length === 0) {
        tbody.innerHTML = '<tr><td colspan="3" class="empty-state"><div class="icon">🌐</div>No domains found</td></tr>';
        document.getElementById('kns-pagination').innerHTML = '';
        return;
    }

    // Paginate
    const start = (knsPage - 1) * PAGE_SIZE;
    const pageData = filtered.slice(start, start + PAGE_SIZE);

    tbody.innerHTML = pageData.map(d => `
                <tr>
                    <td><span class="ticker-badge" style="background:var(--kns-color)">${d.assetId || d.asset_id}</span></td>
                    <td>${formatKAS(d.price)} KAS</td>
                    <td style="font-size:0.75rem;color:var(--text-secondary)">${truncateAddress(d.sellerAddress || d.seller_address)}</td>
                </tr>
            `).join('');

    renderPagination('kns-pagination', knsPage, filtered.length, (page) => {
        knsPage = page;
        filterKNS();
    });
}

// Load Trends Data
async function loadTrendsData() {
    const timeFrame = document.getElementById('trends-timeframe').value;

    try {
        // KRC20 trade stats
        const krc20Url = `${API_BASE}/v1/api/kaspa/trade-stats?timeFrame=${timeFrame}`;
        const krc20Res = await fetch(krc20Url);
        if (krc20Res.ok) {
            const stats = await krc20Res.json();
            document.getElementById('trends-krc20-trades').textContent = formatNumber(stats.totalTradesKaspiano);
            document.getElementById('trends-krc20-volume').textContent = stats.totalVolumeKasKaspiano || '--';

            // Render trades table
            const tradesTbody = document.getElementById('trades-tbody');
            if (stats.tokens && stats.tokens.length > 0) {
                tradesTbody.innerHTML = stats.tokens.slice(0, 20).map(t => `
                            <tr>
                                <td><span class="ticker-badge">${t.ticker}</span></td>
                                <td>${formatNumber(t.totalTrades)}</td>
                                <td>${formatKAS(t.totalVolumeKAS)}</td>
                                <td>${t.totalVolumeUsd || '--'}</td>
                            </tr>
                        `).join('');
            } else {
                tradesTbody.innerHTML = '<tr><td colspan="4" class="empty-state">No trades in this period</td></tr>';
            }
        } else {
            console.error('KRC20 trade stats failed:', krc20Res.status, await krc20Res.text().catch(() => ''));
        }

        // NFT trade stats
        const nftUrl = `${API_BASE}/v1/api/kaspa/krc721/trade-stats?timeFrame=${timeFrame}`;
        const nftRes = await fetch(nftUrl);
        if (nftRes.ok) {
            const stats = await nftRes.json();
            document.getElementById('trends-nft-trades').textContent = formatNumber(stats.totalTradesKaspiano);
            document.getElementById('trends-nft-volume').textContent = stats.totalVolumeKasKaspiano || '--';
        }

        // Hot mints
        loadHotMints('1h');
    } catch (e) {
        console.error('Trends load failed:', e);
        // Show error in UI
        document.getElementById('trends-krc20-trades').textContent = 'Error';
        document.getElementById('trends-krc20-volume').textContent = 'Error';
        document.getElementById('trends-nft-trades').textContent = 'Error';
        document.getElementById('trends-nft-volume').textContent = 'Error';
    }
}


// Helper function to get API name for category
function getApiNameForCategory(category) {
    const apiMap = {
        'tokens': 'kaspa.com/token-info',
        'trade_stats': 'kaspa.com/trade-stats',
        'floor_prices': 'kaspa.com/floor-price',
        'historical': 'kaspa.com/historical',
        'orders': 'kaspa.com/orders',
        'hot_mints': 'kaspa.com/hot-mints',
        'logos': 'kaspa.com/logos',
        'krc721': 'kaspa.com/krc721',
        'kns': 'kaspa.com/kns'
    };
    return apiMap[category] || 'kaspa.com/api';
}

// System Data Loading
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
    tbody.innerHTML = '<tr><td colspan="8" class="loading"><div class="spinner"></div> Analyze System Spirituality...</td></tr>';

    // Status placeholders
    document.getElementById('sys-status').innerHTML = '<span class="loading">...</span>';
    document.getElementById('sys-cache-size').innerHTML = '<span class="loading">...</span>';
    document.getElementById('sys-cache-files').innerHTML = '<span class="loading">...</span>';
    document.getElementById('sys-cache-hits').innerHTML = '<span class="loading">...</span>';

    try {
        // Use Promise.allSettled so one failure doesn't block others
        const [healthResult, cacheResult] = await Promise.allSettled([
            fetch(`${API_BASE}/health`),
            fetch(`${API_BASE}/v1/api/kaspa/cache/stats`)
        ]);

        // 1. Health
        if (healthResult.status === 'fulfilled' && healthResult.value.ok) {
            const health = await healthResult.value.json();
            const statusColor = health.status === 'ok' ? 'var(--positive)' : 'var(--negative)';
            document.getElementById('sys-status').innerHTML = `<span style="color:${statusColor};text-shadow:0 0 10px ${statusColor}">●</span> ${health.status.toUpperCase()}`;
        } else {
            document.getElementById('sys-status').innerHTML = '<span style="color:var(--negative)">●</span> ERROR';
        }

        // 2. Cache Stats
        if (cacheResult.status === 'fulfilled' && cacheResult.value.ok) {
            const cache = await cacheResult.value.json();
            document.getElementById('sys-cache-size').textContent = formatBytes(cache.total_size_bytes);
            document.getElementById('sys-cache-files').textContent = formatNumber(cache.total_keys);

            // Calculate totals from all categories
            let totalRequests = 0;
            let totalHits = 0;
            let totalMisses = 0;
            const catMap = cache.categories || {};

            Object.values(catMap).forEach(cat => {
                totalRequests += Number(cat.requests) || 0;
                totalHits += Number(cat.hits) || 0;
                totalMisses += Number(cat.misses) || 0;
            });

            // Use totalHits from categories, fallback to cache_hits if categories are empty
            const displayHits = totalHits > 0 ? totalHits : (cache.cache_hits || 0);
            const totalHitRate = totalRequests > 0 ? ((totalHits / totalRequests) * 100).toFixed(1) :
                (cache.cache_hits > 0 && totalRequests === 0 ? '100.0' : '0.0');

            document.getElementById('sys-cache-hits').innerHTML = `
                ${formatNumber(displayHits)} 
                <span style="color:var(--text-secondary);font-size:0.75rem;display:block;margin-top:2px">
                    ${totalHitRate}% hit rate (${formatNumber(totalHits)}/${formatNumber(totalRequests)})
                </span>
            `;

            // Also update header cache stats
            document.getElementById('cache-hits-header').textContent = formatNumber(displayHits);
            document.getElementById('cache-hit-rate').textContent = `${totalHitRate}%`;

            // Populate Table
            const categories = Object.entries(catMap).sort((a, b) => (b[1].requests || 0) - (a[1].requests || 0));

            if (categories.length === 0) {
                tbody.innerHTML = '<tr><td colspan="8" class="empty-state">No cache details available</td></tr>';
            } else {
                tbody.innerHTML = categories.map(([cat, stats]) => {
                    const requests = Number(stats.requests) || 0;
                    const hits = Number(stats.hits) || 0;
                    const misses = Number(stats.misses) || 0;
                    const hitRate = requests > 0 ? ((hits / requests) * 100).toFixed(1) : '0.0';

                    // Determine API name based on category
                    const apiName = getApiNameForCategory(cat);


                    return `
                    <tr>
                        <td><span class="ticker-badge" style="background:rgba(73, 234, 196, 0.1);border:1px solid var(--accent);color:var(--accent)">${cat}</span></td>
                        <td style="color:var(--text-primary)">${stats.description}</td>
                        <td>${formatNumber(stats.keys || 0)}</td>
                        <td style="font-family:monospace;color:var(--accent)">${formatBytes(stats.size_bytes || 0)}</td>
                        <td style="color:var(--text-primary);font-weight:${requests > 0 ? 'bold' : 'normal'}">${formatNumber(requests)}</td>
                        <td style="color:var(--positive)">
                            ${formatNumber(hits)} 
                            <span style="color:var(--text-secondary);font-size:0.75rem;display:block">${hitRate}%</span>
                        </td>
                        <td style="color:var(--negative)">${formatNumber(misses)}</td>
                        <td>
                            <span style="color:var(--positive);text-shadow:0 0 5px var(--positive)">Active</span>
                            <span style="color:var(--text-secondary);font-size:0.7rem;display:block;margin-top:2px">${apiName}</span>
                        </td>
                    </tr>
                `;
                }).join('');
            }
        } else {
            document.getElementById('sys-cache-size').textContent = 'Error';
            document.getElementById('sys-cache-files').textContent = 'Error';
            document.getElementById('sys-cache-hits').textContent = 'Error';
            tbody.innerHTML = '<tr><td colspan="8" class="empty-state" style="color:var(--negative)">Failed to load cache statistics</td></tr>';
        }
    } catch (e) {
        console.error('System load failed:', e);
        tbody.innerHTML = `<tr><td colspan="8" class="empty-state" style="color:var(--negative)">System Meditation Interrupted: ${e.message}</td></tr>`;
    }
}

async function loadHotMints(interval) {
    const tbody = document.getElementById('hot-mints-tbody');
    try {
        const res = await fetch(`${API_BASE}/v1/api/kaspa/hot-mints?timeInterval=${interval}`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const data = await res.json();

        if (data.length === 0) {
            tbody.innerHTML = '<tr><td colspan="4" class="empty-state">No hot mints right now</td></tr>';
            return;
        }

        tbody.innerHTML = data.slice(0, 10).map(m => `
                    <tr>
                        <td><span class="ticker-badge">${m.ticker}</span></td>
                        <td class="positive">+${formatNumber(m.changeTotalMints || m.change_total_mints)}</td>
                        <td>${((m.totalMintPercentage || m.total_mint_percentage || 0) * 100).toFixed(1)}%</td>
                        <td>${formatNumber(m.totalHolders || m.total_holders)}</td>
                    </tr>
                `).join('');
    } catch (e) {
        console.error('Hot mints load failed:', e);
        tbody.innerHTML = `<tr><td colspan="4" class="empty-state">Failed to load</td></tr>`;
    }
}

function loadTabData() {
    switch (currentTab) {
        case 'krc20': loadKRC20Data(); break;
        case 'krc721': loadKRC721Data(); break;
        case 'kns': loadKNSData(); break;
        case 'trends': loadTrendsData(); break;
        case 'system': loadSystemData(); break;
    }
}

// ================================================================
// Detail View Functions
// ================================================================

function closeDetail() {
    document.getElementById('detail-overlay').classList.remove('active');
}

// Close on escape key or clicking overlay background
document.getElementById('detail-overlay').addEventListener('click', (e) => {
    if (e.target.id === 'detail-overlay') closeDetail();
});
document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') closeDetail();
});

// Show KRC20 Token Detail
async function showTokenDetail(ticker) {
    const overlay = document.getElementById('detail-overlay');
    overlay.classList.add('active');

    // Set initial state
    document.getElementById('detail-logo').innerHTML = '🪙';
    document.getElementById('detail-name').textContent = ticker;
    document.getElementById('detail-subtitle').textContent = 'KRC20 Token';
    document.getElementById('detail-description').textContent = 'Loading...';
    document.getElementById('detail-stats').innerHTML = '<div class="detail-stat"><div class="detail-stat-label">Loading...</div><div class="detail-stat-value">--</div></div>';
    document.getElementById('detail-links').innerHTML = '';
    document.getElementById('detail-metadata').innerHTML = '';
    document.getElementById('detail-nfts-section').style.display = 'none';
    document.getElementById('detail-description-section').style.display = 'block';

    try {
        // Fetch token info
        const res = await fetch(`${API_BASE}/v1/api/kaspa/token-info/${ticker}`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const info = await res.json();

        // Update logo
        if (info.logoUrl) {
            document.getElementById('detail-logo').innerHTML = `<img src="${info.logoUrl}" alt="${ticker}" onerror="this.parentElement.innerHTML='🪙'">`;
        }

        // Update name and subtitle
        document.getElementById('detail-name').textContent = ticker;
        document.getElementById('detail-subtitle').textContent = info.state === 'finished' ? 'Fully Minted KRC20' : 'KRC20 Token (Minting)';

        // Update stats
        const floorEntry = krc20Data.find(t => t.ticker === ticker);
        document.getElementById('detail-stats').innerHTML = `
                    <div class="detail-stat">
                        <div class="detail-stat-label">Floor Price</div>
                        <div class="detail-stat-value">${formatKAS(floorEntry?.floor_price || info.floorPrice)} KAS</div>
                    </div>
                    <div class="detail-stat">
                        <div class="detail-stat-label">Max Supply</div>
                        <div class="detail-stat-value">${formatNumber(info.maxSupply)}</div>
                    </div>
                    <div class="detail-stat">
                        <div class="detail-stat-label">Total Minted</div>
                        <div class="detail-stat-value">${formatNumber(info.totalMinted)}</div>
                    </div>
                    <div class="detail-stat">
                        <div class="detail-stat-label">Holders</div>
                        <div class="detail-stat-value">${formatNumber(info.totalHolders)}</div>
                    </div>
                    <div class="detail-stat">
                        <div class="detail-stat-label">Mint Limit</div>
                        <div class="detail-stat-value">${formatNumber(info.mintLimit)}</div>
                    </div>
                    <div class="detail-stat">
                        <div class="detail-stat-label">Decimals</div>
                        <div class="detail-stat-value">${info.decimals || 8}</div>
                    </div>
                `;

        // Update description
        const desc = info.metadata?.description || 'No description available.';
        document.getElementById('detail-description').textContent = desc;

        // Update links
        let linksHtml = '';
        if (info.metadata?.website) linksHtml += `<a href="${info.metadata.website}" target="_blank" class="detail-link">🌐 Website</a>`;
        if (info.metadata?.twitter) linksHtml += `<a href="${info.metadata.twitter}" target="_blank" class="detail-link">𝕏 Twitter</a>`;
        if (info.metadata?.telegram) linksHtml += `<a href="${info.metadata.telegram}" target="_blank" class="detail-link">📱 Telegram</a>`;
        if (info.metadata?.discord) linksHtml += `<a href="${info.metadata.discord}" target="_blank" class="detail-link">💬 Discord</a>`;
        linksHtml += `<a href="https://kas.fyi/token/krc20/${ticker}" target="_blank" class="detail-link">📊 kas.fyi</a>`;
        linksHtml += `<a href="https://kaspiano.com/token/${ticker}" target="_blank" class="detail-link">🛒 Kaspiano</a>`;
        document.getElementById('detail-links').innerHTML = linksHtml || '<span style="color:var(--text-secondary)">No links available</span>';

        // Update metadata table
        document.getElementById('detail-metadata').innerHTML = `
                    <tr><td style="color:var(--text-secondary)">Deploy Hash</td><td style="font-size:0.75rem">${info.deployHash || '--'}</td></tr>
                    <tr><td style="color:var(--text-secondary)">State</td><td>${info.state || '--'}</td></tr>
                    <tr><td style="color:var(--text-secondary)">Creation Date</td><td>${info.creationDate ? new Date(info.creationDate).toLocaleString() : '--'}</td></tr>
                    <tr><td style="color:var(--text-secondary)">Total Transfers</td><td>${formatNumber(info.totalTransfers)}</td></tr>
                    <tr><td style="color:var(--text-secondary)">Minted %</td><td>${((info.totalMinted / info.maxSupply) * 100).toFixed(2)}%</td></tr>
                `;
    } catch (e) {
        console.error('Token detail load failed:', e);
        document.getElementById('detail-description').textContent = 'Failed to load token information.';
    }
}

// Show KRC721 Collection Detail
async function showCollectionDetail(ticker) {
    const overlay = document.getElementById('detail-overlay');
    overlay.classList.add('active');

    // Set initial state
    document.getElementById('detail-logo').innerHTML = '🎨';
    document.getElementById('detail-name').textContent = ticker;
    document.getElementById('detail-subtitle').textContent = 'NFT Collection';
    document.getElementById('detail-description').textContent = 'Loading collection info...';
    document.getElementById('detail-stats').innerHTML = '<div class="detail-stat"><div class="detail-stat-label">Loading...</div><div class="detail-stat-value">--</div></div>';
    document.getElementById('detail-links').innerHTML = '';
    document.getElementById('detail-metadata').innerHTML = '';
    document.getElementById('detail-description-section').style.display = 'block';
    document.getElementById('detail-nfts-section').style.display = 'block';
    document.getElementById('detail-nfts').innerHTML = '<div class="loading"><div class="spinner"></div> Loading NFTs...</div>';

    try {
        // Fetch collection trade stats
        const statsRes = await fetch(`${API_BASE}/v1/api/kaspa/krc721/trade-stats?timeFrame=24h&ticker=${ticker}`);
        let stats = {};
        if (statsRes.ok) {
            stats = await statsRes.json();
        }

        // Fetch comprehensive collection info
        const infoRes = await fetch(`${API_BASE}/v1/api/kaspa/krc721/collection/${ticker}`);
        let collectionInfo = {};
        if (infoRes.ok) {
            collectionInfo = await infoRes.json();
        }

        // Get floor price from cached data or collection info
        const floorEntry = krc721Data.find(c => c.ticker === ticker);
        const floorPrice = floorEntry?.floor_price || collectionInfo.price || 0;

        // Update stats
        document.getElementById('detail-stats').innerHTML = `
                    <div class="detail-stat">
                        <div class="detail-stat-label">Floor Price</div>
                        <div class="detail-stat-value">${formatKAS(floorPrice)} KAS</div>
                    </div>
                    <div class="detail-stat">
                        <div class="detail-stat-label">Volume (24h)</div>
                        <div class="detail-stat-value">${stats.totalVolumeKasKaspiano ? formatNumber(stats.totalVolumeKasKaspiano) + ' KAS' : '--'}</div>
                    </div>
                    <div class="detail-stat">
                        <div class="detail-stat-label">Trades (24h)</div>
                        <div class="detail-stat-value">${formatNumber(stats.totalTradesKaspiano || 0)}</div>
                    </div>
                    <div class="detail-stat">
                        <div class="detail-stat-label">Holders</div>
                        <div class="detail-stat-value">${collectionInfo.totalHolders ? formatNumber(collectionInfo.totalHolders) : '--'}</div>
                    </div>
                    <div class="detail-stat">
                        <div class="detail-stat-label">Minted</div>
                        <div class="detail-stat-value">${collectionInfo.totalMinted ? formatNumber(collectionInfo.totalMinted) : '--'} / ${collectionInfo.totalSupply ? formatNumber(collectionInfo.totalSupply) : '∞'}</div>
                    </div>
                `;

        const description = collectionInfo.metadata?.description || `${ticker} is an NFT collection on the Kaspa blockchain.`;
        document.getElementById('detail-description').textContent = description;

        // Display social links if available
        let linksHtml = `
                    <a href="https://kaspiano.com/collection/${ticker}" target="_blank" class="detail-link">🛒 Kaspiano</a>
                    <a href="https://kas.fyi/token/krc721/${ticker}" target="_blank" class="detail-link">📊 kas.fyi</a>
                `;

        if (collectionInfo.metadata) {
            if (collectionInfo.metadata.xUrl) linksHtml += `<a href="${collectionInfo.metadata.xUrl}" target="_blank" class="detail-link">🐦 Twitter</a>`;
            if (collectionInfo.metadata.telegramUrl) linksHtml += `<a href="${collectionInfo.metadata.telegramUrl}" target="_blank" class="detail-link">✈️ Telegram</a>`;
            if (collectionInfo.metadata.discordUrl) linksHtml += `<a href="${collectionInfo.metadata.discordUrl}" target="_blank" class="detail-link">💬 Discord</a>`;
        }

        document.getElementById('detail-links').innerHTML = linksHtml;

        // Fetch NFTs in collection
        const nftsRes = await fetch(`${API_BASE}/v1/api/kaspa/krc721/tokens`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ ticker: ticker, limit: 20 })
        });

        if (nftsRes.ok) {
            const nftsData = await nftsRes.json();
            const nfts = nftsData.items || []; // API uses 'items' not 'tokens'

            if (nfts.length === 0) {
                document.getElementById('detail-nfts').innerHTML = '<div class="empty-state">No NFTs available</div>';
            } else {
                document.getElementById('detail-nfts').innerHTML = nfts.slice(0, 12).map(nft => {
                    // Use optimized CDN URL for images
                    const tickerUpper = ticker.toUpperCase();
                    const imageUrl = `https://cache.krc721.stream/krc721/mainnet/optimized/${tickerUpper}/${nft.tokenId}`;
                    const fallbackUrl = nft.image || 'https://kaspiano.com/assets/krc721-placeholder.png'; // Improved fallback

                    return `
                            <div class="nft-card">
                                <div class="nft-image">
                                    <img src="${imageUrl}" alt="#${nft.tokenId}" 
                                         onerror="this.onerror=null; this.src='${fallbackUrl}';">
                                </div>
                                <div class="nft-info">
                                    <div class="nft-name">#${nft.tokenId}</div>
                                    <div class="nft-price">${nft.isListed ? formatKAS(nft.listingPrice) + ' KAS' : 'Not Listed'}</div>
                                    ${nft.rarityRank ? `<div class="nft-rarity">Rank #${nft.rarityRank}</div>` : ''}
                                </div>
                            </div>
                        `}).join('');
            }
        } else {
            document.getElementById('detail-nfts').innerHTML = '<div class="empty-state">Could not load NFTs</div>';
        }

    } catch (e) {
        console.error('Collection detail load failed:', e);
        document.getElementById('detail-description').textContent = 'Failed to load collection information.';
    }
}

// Initial load
async function initialize() {
    await checkStatus();
    loadTrendsData(); // Load default tab (Trends)

    // Set API base URL in developer tab
    document.querySelectorAll('#api-base-url, .api-base-url-display').forEach(el => {
        el.textContent = API_BASE;
    });

    // Update documentation links to absolute URLs
    const docLink = document.getElementById('link-docs');
    if (docLink) docLink.href = `${API_BASE}/swagger-ui`;

    const openApiLink = document.getElementById('link-openapi');
    if (openApiLink) openApiLink.href = `${API_BASE}/v1/openapi.json`;
}

initialize();

// Auto-refresh every 60s
setInterval(() => {
    checkStatus();
    loadTabData();
}, 60000);