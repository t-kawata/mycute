const esbuild = require('esbuild');
const path = require('path');

async function build() {
    const commonOptions = {
        bundle: true,
        minify: true,
        sourcemap: false,
        target: ['es2020'],
    };

    try {
        // 1. Bundle SDK
        await esbuild.build({
            ...commonOptions,
            entryPoints: [path.join(__dirname, 'src', 'mycute_sdk.ts')],
            outfile: path.join(__dirname, 'dist', 'mycute_sdk.js'),
            format: 'esm', // Injected with type="module"
        });
        console.log('✔ SDK built: dist/mycute_sdk.js');

        // 2. Bundle Service Worker
        await esbuild.build({
            ...commonOptions,
            entryPoints: [path.join(__dirname, 'src', 'service-worker', 'mycute_sw.ts')],
            outfile: path.join(__dirname, 'dist', 'mycute_sw.js'),
            format: 'iife', // Service Workers are usually loaded as classic scripts unless specified
        });
        console.log('✔ Service Worker built: dist/mycute_sw.js');

    } catch (e) {
        console.error('Build failed:', e);
        process.exit(1);
    }
}

build();
