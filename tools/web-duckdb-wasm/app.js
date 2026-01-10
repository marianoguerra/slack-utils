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
        await this.initDuckDB();
        this.bindEvents();
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
            this.updateStatus('DuckDB ready. Load parquet files to begin.', 'success');
        } catch (error) {
            console.error('Failed to initialize DuckDB:', error);
            this.updateStatus(`Failed to initialize DuckDB: ${error.message}`, 'error');
        }
    }

    bindEvents() {
        // File inputs
        document.getElementById('conversations-files').addEventListener('change', (e) => this.loadConversations(e));
        document.getElementById('users-file').addEventListener('change', (e) => this.loadParquet(e, 'users'));
        document.getElementById('channels-file').addEventListener('change', (e) => this.loadParquet(e, 'channels'));

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

    async loadConversations(event) {
        const files = event.target.files;
        if (!files.length) return;

        try {
            this.updateStatus('Loading conversation files...', 'success');

            // Register all files and create a unified view
            const fileNames = [];
            for (const file of files) {
                const arrayBuffer = await file.arrayBuffer();
                const fileName = `conv_${file.name}`;
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

            this.updateStatus(`Loaded ${files.length} conversation file(s) with ${count} messages. Tables: ${[...this.loadedTables].join(', ')}`, 'success');
        } catch (error) {
            console.error('Failed to load conversations:', error);
            this.updateStatus(`Failed to load conversations: ${error.message}`, 'error');
        }
    }

    async loadParquet(event, tableName) {
        const file = event.target.files[0];
        if (!file) return;

        try {
            this.updateStatus(`Loading ${tableName}...`, 'success');

            const arrayBuffer = await file.arrayBuffer();
            await this.db.registerFileBuffer(file.name, new Uint8Array(arrayBuffer));

            // Drop existing table if exists
            await this.conn.query(`DROP TABLE IF EXISTS ${tableName}`);

            // Create table from parquet
            await this.conn.query(`CREATE TABLE ${tableName} AS SELECT * FROM parquet_scan('${file.name}')`);

            this.loadedTables.add(tableName);

            const countResult = await this.conn.query(`SELECT COUNT(*) as count FROM ${tableName}`);
            const count = countResult.toArray()[0].count;

            this.updateStatus(`Loaded ${tableName} with ${count} rows. Tables: ${[...this.loadedTables].join(', ')}`, 'success');
        } catch (error) {
            console.error(`Failed to load ${tableName}:`, error);
            this.updateStatus(`Failed to load ${tableName}: ${error.message}`, 'error');
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
            this.showError(`This query requires: ${missing.join(', ')}. Please load the required parquet files first.`);
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

    updateStatus(message, type) {
        const status = document.getElementById('loaded-status');
        status.textContent = message;
        status.className = `loaded-status ${type}`;
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
