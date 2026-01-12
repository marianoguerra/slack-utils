import { parseArgs } from "util";
import { existsSync, statSync } from "fs";
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
Usage: bun static-file-server.js [options]

Options:
  -p, --path <path>   Base path containing parquet files (default: current directory)
  --port <port>       Server port (default: 3000)
  -h, --help          Show this help message

Required files in base path:
  - users.parquet
  - channels.parquet
  - conversations/    (directory with year=*/week=*/*.parquet structure)

This server serves static files only. The frontend handles discovering
and loading parquet files based on the Hive partition structure.
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

// Bundle static-app.js on startup
console.log("Bundling static-app.js...");
const buildResult = await Bun.build({
    entrypoints: ["./static-app.js"],
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

        // Default to static-files.html
        if (path === "/") {
            path = "/static-files.html";
        }

        // Serve bundled static-app.js from dist
        if (path === "/static-app.js") {
            path = "/dist/static-app.js";
        }

        // Serve parquet files from basePath
        if (path.endsWith(".parquet")) {
            const filePath = join(basePath, path);
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

        // Serve static files from current directory
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

console.log(`\nStatic file server running at http://localhost:${server.port}`);
console.log(`Serving parquet files from: ${basePath}`);
console.log("Press Ctrl+C to stop");
