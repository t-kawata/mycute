#!/usr/bin/env node
/**
 * SSE Streaming Test CLI for Cube Absorb API
 * 
 * Usage:
 *   node test-absorb-stream.js -t <token> -m <chat_model_id> -i <cube_id> -g <memory_group> -c <content>
 * 
 * Options:
 *   -t  JWT Bearer token (required)
 *   -m  chat_model_id (required)
 *   -i  cube_id (required)
 *   -g  memory_group (required)
 *   -c  content to absorb (required)
 *   -h  Show help
 */

import { request } from 'http';

// Parse command line arguments
function parseArgs(args) {
    const result = {};
    for (let i = 0; i < args.length; i++) {
        if (args[i] === '-h') {
            return { help: true };
        }
        if (args[i].startsWith('-') && args[i + 1]) {
            const key = args[i].substring(1);
            result[key] = args[i + 1];
            i++;
        }
    }
    return result;
}

function showHelp() {
    console.log(`
SSE Streaming Test CLI for Cube Absorb API

Usage:
  node test-absorb-stream.js -t <token> -m <chat_model_id> -i <cube_id> -g <memory_group> -c <content>

Options:
  -t  JWT Bearer token (required)
  -m  chat_model_id (required)
  -i  cube_id (required)
  -g  memory_group (required)
  -c  content to absorb (required)
  -h  Show this help

Example:
  node test-absorb-stream.js \\
    -t "eyJhbGciOiJIUzI1NiIsInR..." \\
    -m 1 \\
    -i 1 \\
    -g "test_group" \\
    -c "Analyzing the evolution of programming languages..."
`);
}

function main() {
    const args = parseArgs(process.argv.slice(2));

    if (args.help) {
        showHelp();
        process.exit(0);
    }

    // Validate required arguments
    const required = ['t', 'm', 'i', 'g', 'c'];
    const missing = required.filter(key => !args[key]);
    if (missing.length > 0) {
        console.error(`Error: Missing required options: ${missing.map(k => '-' + k).join(', ')}`);
        console.error('Run with -h for help');
        process.exit(1);
    }

    const requestBody = JSON.stringify({
        chat_model_id: parseInt(args.m, 10),
        chunk_overlap: 16,
        chunk_size: 512,
        content: args.c,
        cube_id: parseInt(args.i, 10),
        memory_group: args.g,
        stream: true
    });

    const options = {
        hostname: '127.0.0.1',
        port: 8888,
        path: '/v1/cubes/absorb',
        method: 'PUT',
        headers: {
            'Accept': 'application/json',
            'Authorization': `Bearer ${args.t}`,
            'Content-Type': 'application/json',
            'Content-Length': Buffer.byteLength(requestBody)
        }
    };

    console.log('\n' + '-'.repeat(60));

    const req = request(options, (res) => {
        if (res.statusCode !== 200) {
            console.error(`HTTP Error: ${res.statusCode}`);
            res.on('data', (chunk) => {
                console.error(chunk.toString());
            });
            return;
        }

        let buffer = '';
        const decoder = new TextDecoder('utf-8');

        res.on('data', (chunk) => {
            // Decode the chunk, streaming true keeps the internal state for multi-byte chars
            buffer += decoder.decode(chunk, { stream: true });

            // Process complete SSE messages
            const lines = buffer.split('\n');
            buffer = lines.pop() || ''; // Keep incomplete line in buffer

            for (const line of lines) {
                if (line.startsWith('data: ')) {
                    const data = line.substring(6).trim();
                    if (data === '[DONE]') {
                        return;
                    }

                    try {
                        const json = JSON.parse(data);
                        const content = json.choices?.[0]?.delta?.content;
                        if (content) {
                            process.stdout.write(content);
                        }
                    } catch (e) {
                        // Ignore JSON parse errors for incomplete chunks
                    }
                }
            }
        });

        res.on('end', () => {
            // Flash any remaining data
            const remaining = decoder.decode();
            if (remaining) {
                // Process any final remaining lines logic if needed, but usually redundant for SSE
            }
            console.log('\n' + '-'.repeat(60));
        });
    });

    req.on('error', (e) => {
        console.error(`Request Error: ${e.message}`);
        process.exit(1);
    });

    req.write(requestBody);
    req.end();
}

main();
