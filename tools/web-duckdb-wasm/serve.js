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

// Get parquet files for a date range
function getParquetFilesForRange(startDate, endDate) {
    const files = [];
    const start = new Date(startDate);
    const end = new Date(endDate);

    const availableWeeks = getAvailableWeeks();

    for (const { year, week } of availableWeeks) {
        // Calculate the Monday of this ISO week
        const weekStart = getDateOfISOWeek(week, year);
        const weekEnd = new Date(weekStart);
        weekEnd.setDate(weekEnd.getDate() + 6);

        // Check if this week overlaps with the requested range
        if (weekEnd >= start && weekStart <= end) {
            const weekPath = join(conversationsPath, `year=${year}`, `week=${String(week).padStart(2, '0')}`);
            if (existsSync(weekPath)) {
                const parquetFiles = readdirSync(weekPath).filter(f => f.endsWith(".parquet"));
                for (const pf of parquetFiles) {
                    files.push({
                        path: join(weekPath, pf),
                        year,
                        week,
                        filename: pf
                    });
                }
            }
        }
    }

    return files;
}

// Get the Monday of an ISO week
function getDateOfISOWeek(week, year) {
    const jan4 = new Date(year, 0, 4);
    const dayOfWeek = jan4.getDay() || 7;
    const monday = new Date(jan4);
    monday.setDate(jan4.getDate() - dayOfWeek + 1 + (week - 1) * 7);
    return monday;
}

// Bundle app.js on startup
console.log("Bundling app.js...");
const buildResult = await Bun.build({
    entrypoints: ["./app.js"],
    outdir: "./dist",
    format: "esm",
    target: "browser",
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

        // API endpoints
        if (path === "/api/users.parquet") {
            return new Response(Bun.file(usersPath), {
                headers: {
                    "Content-Type": "application/octet-stream",
                    "Cross-Origin-Opener-Policy": "same-origin",
                    "Cross-Origin-Embedder-Policy": "require-corp",
                }
            });
        }

        if (path === "/api/channels.parquet") {
            return new Response(Bun.file(channelsPath), {
                headers: {
                    "Content-Type": "application/octet-stream",
                    "Cross-Origin-Opener-Policy": "same-origin",
                    "Cross-Origin-Embedder-Policy": "require-corp",
                }
            });
        }

        if (path === "/api/conversations") {
            const startDate = url.searchParams.get("start");
            const endDate = url.searchParams.get("end");

            if (!startDate || !endDate) {
                return new Response(JSON.stringify({ error: "start and end query parameters required" }), {
                    status: 400,
                    headers: { "Content-Type": "application/json" }
                });
            }

            const files = getParquetFilesForRange(startDate, endDate);
            return new Response(JSON.stringify({ files: files.map(f => ({ year: f.year, week: f.week, filename: f.filename })) }), {
                headers: {
                    "Content-Type": "application/json",
                    "Cross-Origin-Opener-Policy": "same-origin",
                    "Cross-Origin-Embedder-Policy": "require-corp",
                }
            });
        }

        // Serve individual conversation parquet files
        const convMatch = path.match(/^\/api\/conversations\/year=(\d+)\/week=(\d+)\/(.+\.parquet)$/);
        if (convMatch) {
            const [, year, week, filename] = convMatch;
            const filePath = join(conversationsPath, `year=${year}`, `week=${week}`, filename);
            if (existsSync(filePath)) {
                return new Response(Bun.file(filePath), {
                    headers: {
                        "Content-Type": "application/octet-stream",
                        "Cross-Origin-Opener-Policy": "same-origin",
                        "Cross-Origin-Embedder-Policy": "require-corp",
                    }
                });
            }
            return new Response("Not Found", { status: 404 });
        }

        if (path === "/api/available-weeks") {
            const weeks = getAvailableWeeks();
            return new Response(JSON.stringify({ weeks }), {
                headers: {
                    "Content-Type": "application/json",
                    "Cross-Origin-Opener-Policy": "same-origin",
                    "Cross-Origin-Embedder-Policy": "require-corp",
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
                    "Cross-Origin-Opener-Policy": "same-origin",
                    "Cross-Origin-Embedder-Policy": "require-corp",
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
console.log("Press Ctrl+C to stop");
