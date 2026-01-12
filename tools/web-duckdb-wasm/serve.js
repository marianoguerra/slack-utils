import { parseArgs } from "util";
import { existsSync, readdirSync, statSync } from "fs";
import { join, resolve } from "path";

// Parse CLI arguments
const { values } = parseArgs({
    args: Bun.argv.slice(2),
    options: {
        path: {
            type: "string",
            short: "p",
            default: ".",
        },
        port: {
            type: "string",
            default: "3000",
        },
        help: {
            type: "boolean",
            short: "h",
            default: false,
        },
    },
    allowPositionals: true,
});

if (values.help) {
    console.log(`
Usage: bun serve.js [options]

Options:
  -p, --path <path>   Base path containing parquet files (default: current directory)
  --port <port>       Server port (default: 3000)
  -h, --help          Show this help message

Required files in base path:
  - users.parquet
  - channels.parquet
  - conversations/    (directory with year=*/week=*/*.parquet structure)

API endpoints (compatible with slack-archive-server):
  GET /archive/users              - Returns users.parquet
  GET /archive/channels           - Returns channels.parquet
  GET /archive/threads-in-range   - List available year/weeks in date range
      ?from=YYYY-MM-DD&to=YYYY-MM-DD
  GET /archive/threads            - Returns threads.parquet for a specific week
      ?year=YYYY&week=WW
`);
    process.exit(0);
}

const basePath = resolve(values.path);
const port = parseInt(values.port, 10);

// Validate required files exist
const usersPath = join(basePath, "users.parquet");
const channelsPath = join(basePath, "channels.parquet");
const conversationsPath = join(basePath, "conversations");

console.log(`Checking parquet files in: ${basePath}`);

const errors = [];
if (!existsSync(usersPath)) {
    errors.push(`  - users.parquet not found at ${usersPath}`);
}
if (!existsSync(channelsPath)) {
    errors.push(`  - channels.parquet not found at ${channelsPath}`);
}
if (!existsSync(conversationsPath)) {
    errors.push(`  - conversations/ directory not found at ${conversationsPath}`);
} else if (!statSync(conversationsPath).isDirectory()) {
    errors.push(`  - conversations exists but is not a directory`);
}

if (errors.length > 0) {
    console.error("\nError: Required parquet files not found:\n");
    console.error(errors.join("\n"));
    console.error("\nMake sure you have exported data using:");
    console.error("  slack-utils export-users --format parquet");
    console.error("  slack-utils export-channels --format parquet");
    console.error("  slack-utils export-conversations --format parquet");
    console.error("\nOr specify a different path with --path <directory>\n");
    process.exit(1);
}

console.log("All required files found.");

// Get available year/week combinations from conversations directory
function getAvailableWeeks() {
    const weeks = [];
    try {
        const years = readdirSync(conversationsPath).filter(d => d.startsWith("year="));
        for (const yearDir of years) {
            const year = yearDir.replace("year=", "");
            const yearPath = join(conversationsPath, yearDir);
            const weekDirs = readdirSync(yearPath).filter(d => d.startsWith("week="));
            for (const weekDir of weekDirs) {
                const week = weekDir.replace("week=", "");
                weeks.push({ year: parseInt(year), week: parseInt(week) });
            }
        }
    } catch (e) {
        console.error("Error reading conversations directory:", e);
    }
    return weeks.sort((a, b) => a.year !== b.year ? a.year - b.year : a.week - b.week);
}

// Get the Monday of an ISO week
function getDateOfISOWeek(week, year) {
    const jan4 = new Date(year, 0, 4);
    const dayOfWeek = jan4.getDay() || 7;
    const monday = new Date(jan4);
    monday.setDate(jan4.getDate() - dayOfWeek + 1 + (week - 1) * 7);
    return monday;
}

// Get available weeks for a date range (matching slack-archive-server API)
function getWeeksInRange(fromDate, toDate) {
    const available = [];
    const start = new Date(fromDate);
    const end = new Date(toDate);

    const allWeeks = getAvailableWeeks();

    for (const { year, week } of allWeeks) {
        // Calculate the Monday of this ISO week
        const weekStart = getDateOfISOWeek(week, year);
        const weekEnd = new Date(weekStart);
        weekEnd.setDate(weekEnd.getDate() + 6);

        // Check if this week overlaps with the requested range
        if (weekEnd >= start && weekStart <= end) {
            available.push({ year, week });
        }
    }

    return available;
}

// Get threads parquet path for a year/week
function getThreadsPath(year, week) {
    const weekStr = String(week).padStart(2, '0');
    return join(conversationsPath, `year=${year}`, `week=${weekStr}`, "threads.parquet");
}

// Common CORS and security headers
const corsHeaders = {
    "Cross-Origin-Opener-Policy": "same-origin",
    "Cross-Origin-Embedder-Policy": "require-corp",
};

// Bundle app.js on startup
console.log("Bundling app.js...");
const buildResult = await Bun.build({
    entrypoints: ["./app.js"],
    outdir: "./dist",
    format: "esm",
});

if (!buildResult.success) {
    console.error("Build failed:", buildResult.logs);
    process.exit(1);
}
console.log("Bundle complete.");

const server = Bun.serve({
    port,
    async fetch(req) {
        const url = new URL(req.url);
        let path = url.pathname;

        // API endpoints (matching slack-archive-server)
        if (path === "/archive/users") {
            if (!existsSync(usersPath)) {
                return Response.json({ error: `File not found: ${usersPath}` }, { status: 404, headers: corsHeaders });
            }
            return new Response(Bun.file(usersPath), {
                headers: {
                    "Content-Type": "application/octet-stream",
                    "Content-Disposition": "attachment; filename=\"users.parquet\"",
                    ...corsHeaders,
                }
            });
        }

        if (path === "/archive/channels") {
            if (!existsSync(channelsPath)) {
                return Response.json({ error: `File not found: ${channelsPath}` }, { status: 404, headers: corsHeaders });
            }
            return new Response(Bun.file(channelsPath), {
                headers: {
                    "Content-Type": "application/octet-stream",
                    "Content-Disposition": "attachment; filename=\"channels.parquet\"",
                    ...corsHeaders,
                }
            });
        }

        if (path === "/archive/threads-in-range") {
            const fromDate = url.searchParams.get("from");
            const toDate = url.searchParams.get("to");

            if (!fromDate || !toDate) {
                return Response.json(
                    { error: "Missing required query parameters: from and to (YYYY-MM-DD format)" },
                    { status: 400, headers: corsHeaders }
                );
            }

            // Validate date format
            const dateRegex = /^\d{4}-\d{2}-\d{2}$/;
            if (!dateRegex.test(fromDate)) {
                return Response.json(
                    { error: `Invalid 'from' date format: ${fromDate}. Expected YYYY-MM-DD` },
                    { status: 400, headers: corsHeaders }
                );
            }
            if (!dateRegex.test(toDate)) {
                return Response.json(
                    { error: `Invalid 'to' date format: ${toDate}. Expected YYYY-MM-DD` },
                    { status: 400, headers: corsHeaders }
                );
            }

            const available = getWeeksInRange(fromDate, toDate);
            return Response.json({ available }, { headers: corsHeaders });
        }

        if (path === "/archive/threads") {
            const yearParam = url.searchParams.get("year");
            const weekParam = url.searchParams.get("week");

            if (!yearParam || !weekParam) {
                return Response.json(
                    { error: "Missing required query parameters: year and week" },
                    { status: 400, headers: corsHeaders }
                );
            }

            const year = parseInt(yearParam, 10);
            const week = parseInt(weekParam, 10);

            if (isNaN(year)) {
                return Response.json(
                    { error: `Invalid year: ${yearParam}` },
                    { status: 400, headers: corsHeaders }
                );
            }

            if (isNaN(week) || week < 1 || week > 53) {
                return Response.json(
                    { error: `Invalid week: ${weekParam}. Week must be between 1 and 53` },
                    { status: 400, headers: corsHeaders }
                );
            }

            const threadsPath = getThreadsPath(year, week);
            if (!existsSync(threadsPath)) {
                return Response.json(
                    { error: `File not found: ${threadsPath}` },
                    { status: 404, headers: corsHeaders }
                );
            }

            return new Response(Bun.file(threadsPath), {
                headers: {
                    "Content-Type": "application/octet-stream",
                    "Content-Disposition": "attachment; filename=\"threads.parquet\"",
                    ...corsHeaders,
                }
            });
        }

        // Default to index.html
        if (path === "/") {
            path = "/index.html";
        }

        // Serve bundled app.js from dist
        if (path === "/app.js") {
            path = "/dist/app.js";
        }

        // Serve static files
        const filePath = "." + path;

        try {
            const file = Bun.file(filePath);

            // Check if file exists
            if (!(await file.exists())) {
                return new Response("Not Found", { status: 404 });
            }

            // Determine content type
            const contentType = getContentType(filePath);

            return new Response(file, {
                headers: {
                    "Content-Type": contentType,
                    ...corsHeaders,
                }
            });
        } catch (error) {
            return new Response("Internal Server Error", { status: 500 });
        }
    }
});

function getContentType(path) {
    const ext = path.split(".").pop().toLowerCase();
    const types = {
        "html": "text/html",
        "css": "text/css",
        "js": "application/javascript",
        "json": "application/json",
        "wasm": "application/wasm",
        "parquet": "application/octet-stream",
    };
    return types[ext] || "application/octet-stream";
}

console.log(`\nServer running at http://localhost:${server.port}`);
console.log(`Serving parquet files from: ${basePath}`);
console.log("\nAPI endpoints:");
console.log("  GET /archive/users              - Returns users.parquet");
console.log("  GET /archive/channels           - Returns channels.parquet");
console.log("  GET /archive/threads-in-range   - List available year/weeks");
console.log("      ?from=YYYY-MM-DD&to=YYYY-MM-DD");
console.log("  GET /archive/threads            - Returns threads.parquet");
console.log("      ?year=YYYY&week=WW");
console.log("\nPress Ctrl+C to stop");
