import * as duckdb from '@duckdb/duckdb-wasm';

// Sample queries matching run-duckdb-sample-queries and more
const SAMPLE_QUERIES = {
    // Overview Stats
    'total-users': {
        query: `SELECT
    COUNT(*) as total_users,
    SUM(CASE WHEN is_bot THEN 1 ELSE 0 END) as bots,
    SUM(CASE WHEN is_admin THEN 1 ELSE 0 END) as admins
FROM users`,
        requires: ['users']
    },
    'total-channels': {
        query: `SELECT
    COUNT(*) as total_channels,
    SUM(CASE WHEN is_archived THEN 1 ELSE 0 END) as archived,
    SUM(CASE WHEN is_private THEN 1 ELSE 0 END) as private
FROM channels`,
        requires: ['channels']
    },
    'total-messages': {
        query: `SELECT
    COUNT(*) as total_messages,
    SUM(CASE WHEN is_reply THEN 1 ELSE 0 END) as thread_replies,
    COUNT(DISTINCT user) as unique_users,
    COUNT(DISTINCT channel_name) as channels_with_messages
FROM conversations`,
        requires: ['conversations']
    },

    // Channel Stats
    'channels-by-members': {
        query: `SELECT name, num_members, topic
FROM channels
WHERE NOT is_archived
ORDER BY num_members DESC
LIMIT 20`,
        requires: ['channels']
    },
    'channels-by-messages': {
        query: `SELECT channel_name, COUNT(*) as msg_count
FROM conversations
GROUP BY channel_name
ORDER BY msg_count DESC
LIMIT 20`,
        requires: ['conversations']
    },

    // User Activity
    'active-users': {
        query: `SELECT user, COUNT(*) as msg_count
FROM conversations
GROUP BY user
ORDER BY msg_count DESC
LIMIT 20`,
        requires: ['conversations']
    },
    'users-not-bots': {
        query: `SELECT id, name, real_name, display_name, email, tz
FROM users
WHERE NOT is_bot
ORDER BY name
LIMIT 50`,
        requires: ['users']
    },

    // Time-based Stats
    'messages-by-week': {
        query: `SELECT year, week, COUNT(*) as msg_count
FROM conversations
GROUP BY year, week
ORDER BY year DESC, week DESC
LIMIT 20`,
        requires: ['conversations']
    },
    'messages-by-date': {
        query: `SELECT date, COUNT(*) as msg_count
FROM conversations
GROUP BY date
ORDER BY date DESC
LIMIT 30`,
        requires: ['conversations']
    },

    // Thread Activity
    'top-threads': {
        query: `SELECT channel_name, thread_ts, COUNT(*) as reply_count
FROM conversations
WHERE is_reply
GROUP BY channel_name, thread_ts
ORDER BY reply_count DESC
LIMIT 20`,
        requires: ['conversations']
    },
    'thread-stats': {
        query: `SELECT
    COUNT(DISTINCT thread_ts) as total_threads,
    SUM(CASE WHEN is_reply THEN 1 ELSE 0 END) as total_replies,
    ROUND(AVG(reply_count), 2) as avg_replies_per_thread
FROM (
    SELECT thread_ts, COUNT(*) as reply_count
    FROM conversations
    WHERE is_reply
    GROUP BY thread_ts
)`,
        requires: ['conversations']
    },

    // Content Search
    'search-text': {
        query: `SELECT channel_name, user, date, text
FROM conversations
WHERE text LIKE '%KEYWORD%'
ORDER BY date DESC
LIMIT 50`,
        requires: ['conversations']
    },
    'recent-messages': {
        query: `SELECT channel_name, user, date,
    CASE WHEN LENGTH(text) > 100 THEN SUBSTR(text, 1, 100) || '...' ELSE text END as text
FROM conversations
ORDER BY ts DESC
LIMIT 30`,
        requires: ['conversations']
    }
};

class DuckDBApp {
    constructor() {
        this.db = null;
        this.conn = null;
        this.loadedTables = new Set();

        this.init();
    }

    async init() {
        this.setDefaultDates();
        await this.initDuckDB();
        this.bindEvents();
        await this.loadInitialData();
    }

    setDefaultDates() {
        const params = new URLSearchParams(window.location.search);

        // Check for fromYear/fromWeek and toYear/toWeek params
        const fromYear = params.get('fromYear');
        const fromWeek = params.get('fromWeek');
        const toYear = params.get('toYear') || fromYear;
        const toWeek = params.get('toWeek') || fromWeek;

        if (fromYear && fromWeek) {
            const { startDate } = this.getDateRangeForWeek(parseInt(fromYear), parseInt(fromWeek));
            const { endDate } = this.getDateRangeForWeek(parseInt(toYear), parseInt(toWeek));
            document.getElementById('start-date').value = this.formatDate(startDate);
            document.getElementById('end-date').value = this.formatDate(endDate);
            this.autoLoadConversations = true;
            return;
        }

        // Check for fromDate and toDate params
        const fromDate = params.get('fromDate');
        const toDate = params.get('toDate');

        if (fromDate && toDate) {
            document.getElementById('start-date').value = fromDate;
            document.getElementById('end-date').value = toDate;
            this.autoLoadConversations = true;
            return;
        }

        // Default: last 2 months
        const endDate = new Date();
        const startDate = new Date();
        startDate.setMonth(startDate.getMonth() - 2);

        document.getElementById('start-date').value = this.formatDate(startDate);
        document.getElementById('end-date').value = this.formatDate(endDate);
    }

    getDateRangeForWeek(year, week) {
        // Get the Monday of the ISO week
        const jan4 = new Date(year, 0, 4);
        const dayOfWeek = jan4.getDay() || 7;
        const monday = new Date(jan4);
        monday.setDate(jan4.getDate() - dayOfWeek + 1 + (week - 1) * 7);

        // Sunday is 6 days after Monday
        const sunday = new Date(monday);
        sunday.setDate(monday.getDate() + 6);

        return { startDate: monday, endDate: sunday };
    }

    formatDate(date) {
        return date.toISOString().split('T')[0];
    }

    async initDuckDB() {
        try {
            const JSDELIVR_BUNDLES = duckdb.getJsDelivrBundles();
            const bundle = await duckdb.selectBundle(JSDELIVR_BUNDLES);

            const worker_url = URL.createObjectURL(
                new Blob([`importScripts("${bundle.mainWorker}");`], { type: 'text/javascript' })
            );

            const worker = new Worker(worker_url);
            const logger = new duckdb.ConsoleLogger();
            this.db = new duckdb.AsyncDuckDB(logger, worker);

            await this.db.instantiate(bundle.mainModule, bundle.pthreadWorker);
            URL.revokeObjectURL(worker_url);

            this.conn = await this.db.connect();

            console.log('DuckDB initialized successfully');
        } catch (error) {
            console.error('Failed to initialize DuckDB:', error);
            this.showLoadError(`Failed to initialize DuckDB: ${error.message}`);
        }
    }

    bindEvents() {
        // Load conversations button
        document.getElementById('load-conversations').addEventListener('click', () => this.loadConversations());

        // Query controls
        document.getElementById('sample-select').addEventListener('change', (e) => this.selectSampleQuery(e));
        document.getElementById('run-query').addEventListener('click', () => this.runQuery());
        document.getElementById('clear-query').addEventListener('click', () => this.clearQuery());

        // Keyboard shortcut
        document.getElementById('query-input').addEventListener('keydown', (e) => {
            if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
                e.preventDefault();
                this.runQuery();
            }
        });
    }

    async loadInitialData() {
        // Load users and channels on startup
        await Promise.all([
            this.loadFromAPI('users'),
            this.loadFromAPI('channels')
        ]);

        // Auto-load conversations if query params were provided
        if (this.autoLoadConversations) {
            await this.loadConversations();
        }
    }

    async loadFromAPI(tableName) {
        const statusEl = document.getElementById(`${tableName}-status`);
        statusEl.textContent = 'Loading...';
        statusEl.className = 'status-value loading';

        try {
            const response = await fetch(`/api/${tableName}.parquet`);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`);
            }

            const arrayBuffer = await response.arrayBuffer();
            const fileName = `${tableName}.parquet`;
            await this.db.registerFileBuffer(fileName, new Uint8Array(arrayBuffer));

            // Drop existing table if exists
            await this.conn.query(`DROP TABLE IF EXISTS ${tableName}`);

            // Create table from parquet
            await this.conn.query(`CREATE TABLE ${tableName} AS SELECT * FROM parquet_scan('${fileName}')`);

            this.loadedTables.add(tableName);

            const countResult = await this.conn.query(`SELECT COUNT(*) as count FROM ${tableName}`);
            const count = countResult.toArray()[0].count;

            statusEl.textContent = `${count} rows`;
            statusEl.className = 'status-value loaded';
        } catch (error) {
            console.error(`Failed to load ${tableName}:`, error);
            statusEl.textContent = `Error: ${error.message}`;
            statusEl.className = 'status-value error';
        }
    }

    async loadConversations() {
        const startDate = document.getElementById('start-date').value;
        const endDate = document.getElementById('end-date').value;

        if (!startDate || !endDate) {
            this.showLoadError('Please select both start and end dates');
            return;
        }

        const statusEl = document.getElementById('conversations-status');
        const loadBtn = document.getElementById('load-conversations');

        statusEl.textContent = 'Loading...';
        statusEl.className = 'status-value loading';
        loadBtn.disabled = true;
        this.hideLoadError();

        try {
            // Get list of files for the date range
            const listResponse = await fetch(`/api/conversations?start=${startDate}&end=${endDate}`);
            if (!listResponse.ok) {
                throw new Error(`HTTP ${listResponse.status}`);
            }

            const { files } = await listResponse.json();

            if (files.length === 0) {
                statusEl.textContent = 'No data for range';
                statusEl.className = 'status-value';
                loadBtn.disabled = false;
                return;
            }

            // Load each parquet file
            const fileNames = [];
            for (let i = 0; i < files.length; i++) {
                const file = files[i];
                statusEl.textContent = `Loading ${i + 1}/${files.length}...`;

                const url = `/api/conversations/year=${file.year}/week=${String(file.week).padStart(2, '0')}/${file.filename}`;
                const response = await fetch(url);
                if (!response.ok) {
                    throw new Error(`Failed to load ${file.filename}`);
                }

                const arrayBuffer = await response.arrayBuffer();
                const fileName = `conv_${file.year}_${file.week}_${file.filename}`;
                await this.db.registerFileBuffer(fileName, new Uint8Array(arrayBuffer));
                fileNames.push(fileName);
            }

            // Drop existing table if exists
            await this.conn.query('DROP TABLE IF EXISTS conversations');

            // Create table from all parquet files using UNION ALL
            if (fileNames.length === 1) {
                await this.conn.query(`CREATE TABLE conversations AS SELECT * FROM parquet_scan('${fileNames[0]}')`);
            } else {
                const unions = fileNames.map(f => `SELECT * FROM parquet_scan('${f}')`).join(' UNION ALL ');
                await this.conn.query(`CREATE TABLE conversations AS ${unions}`);
            }

            this.loadedTables.add('conversations');

            const countResult = await this.conn.query('SELECT COUNT(*) as count FROM conversations');
            const count = countResult.toArray()[0].count;

            statusEl.textContent = `${count} messages (${files.length} files)`;
            statusEl.className = 'status-value loaded';
        } catch (error) {
            console.error('Failed to load conversations:', error);
            statusEl.textContent = 'Error';
            statusEl.className = 'status-value error';
            this.showLoadError(`Failed to load conversations: ${error.message}`);
        } finally {
            loadBtn.disabled = false;
        }
    }

    selectSampleQuery(event) {
        const queryId = event.target.value;
        if (!queryId) return;

        const sample = SAMPLE_QUERIES[queryId];
        if (!sample) return;

        // Check if required tables are loaded
        const missing = sample.requires.filter(t => !this.loadedTables.has(t));
        if (missing.length > 0) {
            this.showError(`This query requires: ${missing.join(', ')}. Please load the required data first.`);
        }

        document.getElementById('query-input').value = sample.query;
    }

    async runQuery() {
        const queryInput = document.getElementById('query-input');
        const query = queryInput.value.trim();

        if (!query) {
            this.showError('Please enter a query');
            return;
        }

        this.showLoading(true);
        this.hideError();

        try {
            const startTime = performance.now();
            const result = await this.conn.query(query);
            const endTime = performance.now();

            const rows = result.toArray();
            const schema = result.schema;

            this.displayResults(rows, schema, endTime - startTime);
        } catch (error) {
            console.error('Query failed:', error);
            this.showError(error.message);
        } finally {
            this.showLoading(false);
        }
    }

    displayResults(rows, schema, executionTime) {
        const thead = document.getElementById('results-head');
        const tbody = document.getElementById('results-body');
        const countSpan = document.getElementById('result-count');

        // Clear previous results
        thead.innerHTML = '';
        tbody.innerHTML = '';

        if (rows.length === 0) {
            countSpan.textContent = '(0 rows)';
            tbody.innerHTML = '<tr><td colspan="100" style="text-align: center; color: var(--text-muted);">No results</td></tr>';
            return;
        }

        // Update count
        countSpan.textContent = `(${rows.length} rows, ${executionTime.toFixed(2)}ms)`;

        // Build header
        const headerRow = document.createElement('tr');
        for (const field of schema.fields) {
            const th = document.createElement('th');
            th.textContent = field.name;
            headerRow.appendChild(th);
        }
        thead.appendChild(headerRow);

        // Build body
        for (const row of rows) {
            const tr = document.createElement('tr');
            for (const field of schema.fields) {
                const td = document.createElement('td');
                const value = row[field.name];
                td.textContent = value === null ? 'NULL' : String(value);
                td.title = td.textContent; // Show full text on hover
                tr.appendChild(td);
            }
            tbody.appendChild(tr);
        }
    }

    clearQuery() {
        document.getElementById('query-input').value = '';
        document.getElementById('sample-select').value = '';
        document.getElementById('results-head').innerHTML = '';
        document.getElementById('results-body').innerHTML = '';
        document.getElementById('result-count').textContent = '';
        this.hideError();
    }

    showLoadError(message) {
        const error = document.getElementById('load-error');
        error.textContent = message;
        error.classList.remove('hidden');
    }

    hideLoadError() {
        document.getElementById('load-error').classList.add('hidden');
    }

    showLoading(show) {
        document.getElementById('loading').classList.toggle('hidden', !show);
        document.getElementById('run-query').disabled = show;
    }

    showError(message) {
        const error = document.getElementById('error');
        error.textContent = message;
        error.classList.remove('hidden');
    }

    hideError() {
        document.getElementById('error').classList.add('hidden');
    }
}

// Initialize app when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    new DuckDBApp();
});
