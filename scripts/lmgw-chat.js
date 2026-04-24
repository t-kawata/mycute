#!/usr/bin/env node

const ENDPOINT = 'http://localhost:3910/v1/lmgw/v1/chat/completions';
const AUTH_TOKEN = 'eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhcHhfaWQiOjEsInZkcl9pZCI6MiwidXNyX2lkIjozLCJlbWFpbCI6Imthd2F0YUBzaHltZS5uZXQiLCJ0eXBlIjowLCJpc19zdGFmZiI6ZmFsc2UsImV4cCI6MTc3NzA4ODI4NH0.YhfcBIioZg50K-cWgvIyHZC77lgVR77m95HRMC6T1TQ';
const MODEL = 'openai/gpt-4.1-nano';
const BAR_WIDTH = 72;

function printHelp() {
  console.log(`Usage: node lmgw-chat.js -m "message" [-s on|off]

Options:
  -m    Message to send (required)
  -s    Stream mode: on/off (optional, default: off)

Examples:
  node lmgw-chat.js -m "こんにちは。元気ですか？"
  node lmgw-chat.js -m "こんにちは。元気ですか？" -s on
`);
}

function parseArgs(argv) {
  let message = '';
  let stream = false;

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];

    if (arg === '-m') {
      message = argv[++i] ?? '';
    } else if (arg === '-s') {
      const value = (argv[++i] ?? '').toLowerCase();
      if (value !== 'on' && value !== 'off') {
        throw new Error('Invalid value for -s. Use on or off.');
      }
      stream = value === 'on';
    } else if (arg === '-h' || arg === '--help') {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  if (!message) {
    throw new Error('Message is required. Use -m "your message".');
  }

  return { message, stream };
}

function getBar() {
  const width = process.stdout.columns && process.stdout.columns > 0
    ? process.stdout.columns
    : BAR_WIDTH;
  return '-'.repeat(width);
}

function writeTopBar() {
  process.stdout.write(getBar() + '\n');
}

function writeBottomBar() {
  process.stdout.write('\n' + getBar() + '\n');
}

function extractContentFromResponse(data) {
  return data?.choices?.[0]?.message?.content ?? '';
}

function extractContentFromStreamEvent(data) {
  const delta = data?.choices?.[0]?.delta;
  if (!delta) return '';

  if (typeof delta.content === 'string') return delta.content;

  if (Array.isArray(delta.content)) {
    return delta.content
      .map((item) => {
        if (typeof item === 'string') return item;
        if (item?.type === 'text' && typeof item.text === 'string') return item.text;
        return '';
      })
      .join('');
  }

  return '';
}

async function handleNonStream(response) {
  const data = await response.json();
  const content = extractContentFromResponse(data);

  writeTopBar();

  if (content) {
    process.stdout.write(content.endsWith('\n') ? content.slice(0, -1) : content);
    writeBottomBar();
    return;
  }

  process.stdout.write(JSON.stringify(data, null, 2));
  writeBottomBar();
}

async function handleStream(response) {
  if (!response.body) {
    throw new Error('ReadableStream is not available in this environment.');
  }

  writeTopBar();

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split(/\r?\n/);
    buffer = lines.pop() ?? '';

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed || !trimmed.startsWith('data:')) continue;

      const payload = trimmed.slice(5).trim();
      if (payload === '[DONE]') continue;

      try {
        const data = JSON.parse(payload);
        const content = extractContentFromStreamEvent(data);
        if (content) {
          process.stdout.write(content);
        }
      } catch {
      }
    }
  }

  if (buffer.trim().startsWith('data:')) {
    const payload = buffer.trim().slice(5).trim();
    if (payload && payload !== '[DONE]') {
      try {
        const data = JSON.parse(payload);
        const content = extractContentFromStreamEvent(data);
        if (content) {
          process.stdout.write(content);
        }
      } catch {
      }
    }
  }

  writeBottomBar();
}

async function main() {
  try {
    const { message, stream } = parseArgs(process.argv.slice(2));

    const payload = {
      model: MODEL,
      messages: [
        { role: 'user', content: message }
      ],
      ...(stream ? { stream: true } : {})
    };

    const response = await fetch(ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${AUTH_TOKEN}`,
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.error(`HTTP ${response.status} ${response.statusText}`);
      console.error(errorText);
      process.exit(1);
    }

    if (stream) {
      await handleStream(response);
    } else {
      await handleNonStream(response);
    }
  } catch (error) {
    console.error(error.message);
    printHelp();
    process.exit(1);
  }
}

main();
