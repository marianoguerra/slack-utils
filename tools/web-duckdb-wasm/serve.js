const server = Bun.serve({
    port: 3000,
    async fetch(req) {
        const url = new URL(req.url);
        let path = url.pathname;

        // Default to index.html
        if (path === '/') {
            path = '/index.html';
        }

        // Serve static files
        const filePath = '.' + path;

        try {
            const file = Bun.file(filePath);

            // Check if file exists
            if (!(await file.exists())) {
                return new Response('Not Found', { status: 404 });
            }

            // Determine content type
            const contentType = getContentType(filePath);

            return new Response(file, {
                headers: {
                    'Content-Type': contentType,
                    'Cross-Origin-Opener-Policy': 'same-origin',
                    'Cross-Origin-Embedder-Policy': 'require-corp',
                }
            });
        } catch (error) {
            return new Response('Internal Server Error', { status: 500 });
        }
    }
});

function getContentType(path) {
    const ext = path.split('.').pop().toLowerCase();
    const types = {
        'html': 'text/html',
        'css': 'text/css',
        'js': 'application/javascript',
        'json': 'application/json',
        'wasm': 'application/wasm',
        'parquet': 'application/octet-stream',
    };
    return types[ext] || 'application/octet-stream';
}

console.log(`Server running at http://localhost:${server.port}`);
console.log('Press Ctrl+C to stop');
