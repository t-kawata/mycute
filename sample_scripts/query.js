#!/usr/bin/env node
/**
 * SSE Streaming Test CLI for Cube Query API
 * 
 * Usage:
 *   node query.js -t <token> -m <chat_model_id> -i <cube_id> -g <memory_group> -q <query_text> [-T <type>] [-j]
 * 
 * Options:
 *   -t  JWT Bearer token (required)
 *   -m  chat_model_id (required)
 *   -i  cube_id (required)
 *   -g  memory_group (required)
 *   -q  query text (required)
 *   -T  query type (1-11, default: 11, see API docs for details)
 *   -j  as_json mode (output final result as JSON instead of readable text)
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
        if (args[i] === '-j') {
            result['j'] = true;
            continue;
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
SSE Streaming Test CLI for Cube Query API

Usage:
  node query.js -t <token> -m <chat_model_id> -i <cube_id> -g <memory_group> -q <query_text> [-T <type>] [-j]

Options:
  -t  JWT Bearer token (required)
  -m  chat_model_id (required)
  -i  cube_id (required)
  -g  memory_group (required)
  -q  query text (required)
  -T  query type (1-11, default: 11)
      1:  GET_GRAPH
      2:  GET_CHUNKS
      3:  GET_PRE_MADE_SUMMARIES
      4:  GET_GRAPH_AND_CHUNKS
      5:  GET_GRAPH_AND_PRE_MADE_SUMMARIES
      6:  GET_GRAPH_AND_CHUNKS_AND_PRE_MADE_SUMMARIES
      7:  GET_GRAPH_EXPLANATION
      8:  GET_GRAPH_SUMMARY
      9:  GET_GRAPH_SUMMARY_TO_ANSWER
      10: ANSWER_BY_PRE_MADE_SUMMARIES_AND_GRAPH_SUMMARY
      11: ANSWER_BY_CHUNKS_AND_GRAPH_SUMMARY (default)
  -j  as_json mode (output final result as JSON instead of readable text)
  -h  Show this help

Example:
  node query.js \\
    -t "eyJhbGciOiJIUzI1NiIsInR..." \\
    -m 1 \\
    -i 1 \\
    -g "test_group" \\
    -q "契約違反の場合の対処法は？"

  # With JSON output:
  node query.js -t "xxx" -m 1 -i 1 -g "test_group" -q "質問" -j
`);
}

function main() {
    const args = parseArgs(process.argv.slice(2));

    if (args.help) {
        showHelp();
        process.exit(0);
    }

    // Validate required arguments
    const required = ['t', 'm', 'i', 'g', 'q'];
    const missing = required.filter(key => !args[key]);
    if (missing.length > 0) {
        console.error(`Error: Missing required options: ${missing.map(k => '-' + k).join(', ')}`);
        console.error('Run with -h for help');
        process.exit(1);
    }

    const queryType = parseInt(args.T || '11', 10);
    const asJson = !!args.j;

    const requestBody = JSON.stringify({
        cube_id: parseInt(args.i, 10),
        memory_group: args.g,
        text: args.q,
        type: queryType,
        summary_topk: 3,
        chunk_topk: 3,
        entity_topk: 3,
        fts_type: 0,
        fts_topk: 0,
        thickness_threshold: 0.3,
        conflict_resolution_stage: 2,
        chat_model_id: parseInt(args.m, 10),
        stream: true,
        as_json: asJson,
        is_en: false
    });

    const options = {
        hostname: '127.0.0.1',
        port: 8888,
        path: '/v1/cubes/query',
        method: 'POST',
        headers: {
            'Accept': 'application/json',
            'Authorization': `Bearer ${args.t}`,
            'Content-Type': 'application/json',
            'Content-Length': Buffer.byteLength(requestBody)
        }
    };

    console.log('\n' + '='.repeat(60));
    console.log(`Query: "${args.q}"`);
    console.log(`Type: ${queryType}, AsJson: ${asJson}`);
    console.log('='.repeat(60) + '\n');

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
            // Flush any remaining data
            const remaining = decoder.decode();
            if (remaining) {
                // Process any final remaining lines if needed
            }
            console.log('\n' + '='.repeat(60));
            console.log('Query stream completed.');
            console.log('='.repeat(60));
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
