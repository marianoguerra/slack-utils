import { SlackArchiveClient, SlackArchiveDuckDB } from 'slack-archive-client';

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
FROM threads`,
        requires: ['threads']
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
FROM threads
GROUP BY channel_name
ORDER BY msg_count DESC
LIMIT 20`,
        requires: ['threads']
    },

    // User Activity
    'active-users': {
        query: `SELECT user, COUNT(*) as msg_count
FROM threads
GROUP BY user
ORDER BY msg_count DESC
LIMIT 20`,
        requires: ['threads']
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
FROM threads
GROUP BY year, week
ORDER BY year DESC, week DESC
LIMIT 20`,
        requires: ['threads']
    },
    'messages-by-date': {
        query: `SELECT date, COUNT(*) as msg_count
FROM threads
GROUP BY date
ORDER BY date DESC
LIMIT 30`,
        requires: ['threads']
    },

    // Thread Activity
    'top-threads': {
        query: `SELECT channel_name, thread_ts, COUNT(*) as reply_count
FROM threads
WHERE is_reply
GROUP BY channel_name, thread_ts
ORDER BY reply_count DESC
LIMIT 20`,
        requires: ['threads']
    },
    'thread-stats': {
        query: `SELECT
    COUNT(DISTINCT thread_ts) as total_threads,
    SUM(CASE WHEN is_reply THEN 1 ELSE 0 END) as total_replies,
    ROUND(AVG(reply_count), 2) as avg_replies_per_thread
FROM (
    SELECT thread_ts, COUNT(*) as reply_count
    FROM threads
    WHERE is_reply
    GROUP BY thread_ts
)`,
        requires: ['threads']
    },

    // Content Search
    'search-text': {
        query: `SELECT channel_name, user, date, text
FROM threads
WHERE text LIKE '%KEYWORD%'
ORDER BY date DESC
LIMIT 50`,
        requires: ['threads']
    },
    'recent-messages': {
        query: `SELECT channel_name, user, date,
    CASE WHEN LENGTH(text) > 100 THEN SUBSTR(text, 1, 100) || '...' ELSE text END as text
FROM threads
ORDER BY ts DESC
LIMIT 30`,
        requires: ['threads']
    }
};

class DuckDBApp {
    constructor() {
        this.db = null;
        this.client = null;

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
            this.autoLoadThreads = true;
            return;
        }

        // Check for fromDate and toDate params
        const fromDate = params.get('fromDate');
        const toDate = params.get('toDate');

        if (fromDate && toDate) {
            document.getElementById('start-date').value = fromDate;
            document.getElementById('end-date').value = toDate;
            this.autoLoadThreads = true;
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
            // Create SlackArchiveClient pointing to the current server
            this.client = new SlackArchiveClient({
                baseUrl: window.location.origin
            });

            // Create and initialize SlackArchiveDuckDB
            this.db = new SlackArchiveDuckDB({ client: this.client });
            await this.db.init();

            console.log('DuckDB initialized successfully');
        } catch (error) {
            console.error('Failed to initialize DuckDB:', error);
            this.showLoadError(`Failed to initialize DuckDB: ${error.message}`);
        }
    }

    bindEvents() {
        // Load threads button
        document.getElementById('load-conversations').addEventListener('click', () => this.loadThreads());

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
            this.loadTable('users'),
            this.loadTable('channels')
        ]);

        // Auto-load threads if query params were provided
        if (this.autoLoadThreads) {
            await this.loadThreads();
        }
    }

    async loadTable(tableName) {
        const statusEl = document.getElementById(`${tableName}-status`);
        statusEl.textContent = 'Loading...';
        statusEl.className = 'status-value loading';

        try {
            let rowCount;
            if (tableName === 'users') {
                rowCount = await this.db.loadUsers();
            } else if (tableName === 'channels') {
                rowCount = await this.db.loadChannels();
            }

            statusEl.textContent = `${rowCount} rows`;
            statusEl.className = 'status-value loaded';
        } catch (error) {
            console.error(`Failed to load ${tableName}:`, error);
            statusEl.textContent = `Error: ${error.message}`;
            statusEl.className = 'status-value error';
        }
    }

    async loadThreads() {
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
            let partitionCount = 0;

            const rowCount = await this.db.loadThreads(startDate, endDate, (progress) => {
                partitionCount = progress.total;
                statusEl.textContent = progress.message;
            });

            if (rowCount === 0) {
                statusEl.textContent = 'No data for range';
                statusEl.className = 'status-value';
            } else {
                statusEl.textContent = `${rowCount} messages (${partitionCount} files)`;
                statusEl.className = 'status-value loaded';
            }
        } catch (error) {
            console.error('Failed to load threads:', error);
            statusEl.textContent = 'Error';
            statusEl.className = 'status-value error';
            this.showLoadError(`Failed to load threads: ${error.message}`);
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
        const loadedTables = this.db.getLoadedTables();
        const missing = sample.requires.filter(t => !loadedTables.has(t));
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
            const result = await this.db.query(query);
            this.displayResults(result.rows, result.schema, result.executionTimeMs);
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
        for (const field of schema) {
            const th = document.createElement('th');
            th.textContent = field.name;
            headerRow.appendChild(th);
        }
        thead.appendChild(headerRow);

        // Build body
        for (const row of rows) {
            const tr = document.createElement('tr');
            for (const field of schema) {
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
