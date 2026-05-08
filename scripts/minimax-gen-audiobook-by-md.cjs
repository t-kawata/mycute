#!/usr/bin/env node
const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const API_KEY = "YOUR_MINIMAX_API_KEY";
const GROUP_ID = "YOUR_MINIMAX_GROUP_ID";

const MODEL = "speech-2.8-turbo";
const VOICE_ID = "Japanese_refined_storyteller_vv1";
const API_URL = `https://api.minimax.io/v1/t2a_v2?GroupId=${GROUP_ID}`;

const RETRY_MAX = 6;
const RETRY_BASE_MS = 1500;
const REQUEST_TIMEOUT_MS = 180000;

const AUDIO_FORMAT = "mp3";
const SAMPLE_RATE = 32000;
const BITRATE = 128000;
const CHANNEL = 1;

const HEADING_PRE_SILENCE_SEC = 2.0;
const HEADING_POST_SILENCE_SEC = 2.0;
const HEADING_INLINE_PAUSE_SEC = 1.0;

const MAX_TEXT_LENGTH = 9999;
const TARGET_SPLIT_LENGTH = 9200;

function parseArgs(argv) {
  const args = { m: "", d: "", n: 0, dryRun: false };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === "-m") {
      args.m = argv[++i] || "";
    } else if (a === "-d") {
      args.d = argv[++i] || "";
    } else if (a === "-n") {
      const raw = argv[++i];
      const parsed = Number(raw);
      args.n = Number.isFinite(parsed) && parsed >= 0 ? Math.floor(parsed) : 0;
    } else if (a === "-D" || a === "--dry-run") {
      args.dryRun = true;
    } else if (a === "-h" || a === "--help") {
      printHelpAndExit(0);
    } else {
      console.warn(`Unknown argument ignored: ${a}`);
    }
  }

  if (!args.m || !args.d) {
    printHelpAndExit(1);
  }
  return args;
}

function printHelpAndExit(code) {
  const msg = `Usage:
  node minimax-gen-audiobook-by-md.cjs -m input.md -d output_dir [-n 0] [-D]

Options:
  -m   Markdown file path
  -d   Output directory for mp3 files
  -n   Number of chunks to synthesize for testing
       0 or omitted = synthesize all chunks
  -D   Dry run: only show chunk segmentation and lengths, no API calls
`;
  console.log(msg);
  process.exit(code);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function decodeHtmlEntities(text) {
  return text
    .replace(/&nbsp;/g, " ")
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'");
}

function normalizeText(text) {
  return decodeHtmlEntities(text)
    .replace(/\r\n/g, "\n")
    .replace(/\r/g, "\n")
    .replace(/[\t\u00A0]/g, " ")
    .replace(/[ ]{2,}/g, " ")
    .trim();
}

function stripInlineMarkdown(text) {
  return text
    .replace(/!\[[^\]]*\]\([^)]*\)/g, " ")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/__([^_]+)__/g, "$1")
    .replace(/\*([^*]+)\*/g, "$1")
    .replace(/_([^_]+)_/g, "$1")
    .replace(/~~([^~]+)~~/g, "$1")
    .replace(/<[^>]+>/g, " ")
    .replace(/\\([\\`*_{}\[\]()#+\-.!])/g, "$1");
}

function isHeadingBlock(block) {
  const lines = block.split("\n").filter((l) => l.trim().length > 0);
  if (lines.length === 0) return false;
  const first = lines[0];
  return /^\s{0,3}#{1,6}\s+/.test(first);
}

function cleanHeadingTextFromBlock(block) {
  const lines = block.split("\n").filter((l) => l.trim().length > 0);
  if (lines.length === 0) return "";
  let text = lines[0].replace(/^\s{0,3}#{1,6}\s*/, "");
  text = text.replace(/\s+#+\s*$/, "");
  text = stripInlineMarkdown(text);
  text = normalizeText(text);
  return text;
}

function applyHeadingInlinePauseRules(text) {
  let s = text;
  s = s.replace(
    /\s*[:：]\s*/g,
    `。<#${HEADING_INLINE_PAUSE_SEC.toFixed(1)}#> `,
  );
  s = s.replace(/\s+/g, " ").trim();
  return s;
}

function cleanParagraphBlock(block) {
  let s = block;
  s = s.replace(/^\s{0,3}#{1,6}\s+.*$/gm, " ");
  s = s.replace(/^```[\s\S]*?^```\s*$/gm, " ");
  s = s.replace(/^~~~[\s\S]*?^~~~\s*$/gm, " ");
  s = stripInlineMarkdown(s);
  s = s.replace(/\n+/g, " ");
  s = normalizeText(s);
  return s;
}

/**
 * できるだけ自然な位置で maxLen 未満に収まるように分割する。
 */
function findLastBreakBeforeLimit(text, hardLimit, targetLimit) {
  const limit = Math.min(hardLimit, text.length);
  const target = Math.min(targetLimit, limit);

  const priorityPatterns = [
    /[。！？!?]+[」』】）)]?/g,
    /[\.．]+[」』】）)]?/g,
    /[、，,][」』】）)]?/g,
    /[」』】）)]/g,
    /\s+/g,
  ];

  for (const pattern of priorityPatterns) {
    pattern.lastIndex = 0;
    let match;
    let candidate = -1;

    while ((match = pattern.exec(text)) !== null) {
      const end = match.index + match[0].length;
      if (end <= limit) {
        candidate = end;
      } else {
        break;
      }
    }

    if (candidate >= Math.floor(target * 0.6)) {
      return candidate;
    }
  }

  return -1;
}

function splitLongTextNaturally(
  text,
  maxLen = MAX_TEXT_LENGTH,
  targetLen = TARGET_SPLIT_LENGTH,
) {
  const normalized = normalizeText(text);
  if (!normalized) return [];
  if (normalized.length <= maxLen) return [normalized];

  const result = [];
  let rest = normalized;

  while (rest.length > maxLen) {
    let cut = findLastBreakBeforeLimit(rest, maxLen, targetLen);

    if (cut === -1 || cut <= 0) {
      cut = maxLen;
    }

    let part = rest.slice(0, cut).trim();
    rest = rest.slice(cut).trim();

    if (!part) {
      part = rest.slice(0, maxLen).trim();
      rest = rest.slice(maxLen).trim();
    }

    result.push(part);
  }

  if (rest) {
    result.push(rest);
  }

  return result;
}

/**
 * 要件どおり:
 * - 見出し: #〜###### で始まる行を含むブロック → heading チャンク
 * - 段落: 2つ以上の改行で区切ったブロック → paragraph チャンク
 */
function splitMarkdownIntoChunks(markdown) {
  const unified = markdown.replace(/\r\n/g, "\n").replace(/\r/g, "\n");

  // 2つ以上の連続改行でブロック分割
  const rawBlocks = unified.split(/\n{2,}/);

  const chunks = [];

  for (const rawBlock of rawBlocks) {
    const block = rawBlock.trim();
    if (!block) continue;

    if (isHeadingBlock(block)) {
      const headingText = cleanHeadingTextFromBlock(block);
      if (!headingText) continue;
      const headingWithPause = applyHeadingInlinePauseRules(headingText);
      chunks.push({
        type: "heading",
        text: headingWithPause,
        source: block,
        plainHeading: headingText,
      });
    } else {
      const base = cleanParagraphBlock(block);
      if (!base) continue;
      const pieces = splitLongTextNaturally(base);
      for (const p of pieces) {
        chunks.push({
          type: "paragraph",
          text: p,
          source: block,
        });
      }
    }
  }

  return chunks;
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function chunkFilename(index) {
  return `${String(index).padStart(4, "0")}.mp3`;
}

async function fetchWithTimeout(url, options, timeoutMs) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { ...options, signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

function buildPayload(text) {
  const len = text.length;
  if (len >= MAX_TEXT_LENGTH) {
    throw new Error(`Text chunk too long for MiniMax TTS API: ${len} chars`);
  }

  return {
    model: MODEL,
    text,
    stream: false,
    language_boost: "Japanese",
    output_format: "hex",
    voice_setting: {
      voice_id: VOICE_ID,
      speed: 1,
      vol: 1,
      pitch: 0,
    },
    audio_setting: {
      sample_rate: SAMPLE_RATE,
      bitrate: BITRATE,
      format: AUDIO_FORMAT,
      channel: CHANNEL,
    },
  };
}

async function synthesizeChunk(text, index, total) {
  const payload = buildPayload(text);
  let lastError = null;

  for (let attempt = 1; attempt <= RETRY_MAX; attempt++) {
    try {
      console.log(`[${index}/${total}] request attempt ${attempt}`);

      const res = await fetchWithTimeout(
        API_URL,
        {
          method: "POST",
          headers: {
            Authorization: `Bearer ${API_KEY}`,
            "Content-Type": "application/json",
          },
          body: JSON.stringify(payload),
        },
        REQUEST_TIMEOUT_MS,
      );

      const bodyText = await res.text();
      let data;
      try {
        data = JSON.parse(bodyText);
      } catch {
        throw new Error(`Invalid JSON response: ${bodyText.slice(0, 400)}`);
      }

      if (!res.ok) {
        throw new Error(`HTTP ${res.status}: ${bodyText.slice(0, 400)}`);
      }

      if (!data?.base_resp || data.base_resp.status_code !== 0) {
        throw new Error(
          `MiniMax API error: status_code=${data?.base_resp?.status_code}, status_msg=${data?.base_resp?.status_msg || "unknown"}`,
        );
      }

      const hexAudio = data?.data?.audio;
      if (!hexAudio || typeof hexAudio !== "string") {
        throw new Error("Response does not contain data.audio hex string");
      }

      const audioBuffer = Buffer.from(hexAudio, "hex");
      if (!audioBuffer.length) {
        throw new Error("Decoded audio buffer is empty");
      }

      return { audioBuffer, response: data };
    } catch (err) {
      lastError = err;
      console.error(
        `[${index}/${total}] attempt ${attempt} failed: ${err.message}`,
      );

      if (attempt >= RETRY_MAX) break;

      const backoff = RETRY_BASE_MS * Math.pow(2, attempt - 1);
      const jitter = Math.floor(Math.random() * 500);
      const waitMs = backoff + jitter;
      console.log(`[${index}/${total}] retrying after ${waitMs} ms`);
      await sleep(waitMs);
    }
  }

  throw lastError;
}

function requireFfmpeg() {
  const result = spawnSync("ffmpeg", ["-version"], { encoding: "utf8" });
  if (result.error || result.status !== 0) {
    throw new Error(
      "ffmpeg is required to add guaranteed heading silence. Please install ffmpeg and ensure it is in PATH.",
    );
  }
}

function addSilenceWithFfmpeg(inputPath, outputPath, preSec, postSec) {
  const args = [
    "-y",
    "-f",
    "lavfi",
    "-i",
    `anullsrc=r=${SAMPLE_RATE}:cl=mono`,
    "-i",
    inputPath,
    "-f",
    "lavfi",
    "-i",
    `anullsrc=r=${SAMPLE_RATE}:cl=mono`,
    "-filter_complex",
    `[0:a]atrim=0:${preSec}[pre];[2:a]atrim=0:${postSec}[post];[pre][1:a][post]concat=n=3:v=0:a=1[out]`,
    "-map",
    "[out]",
    "-ar",
    String(SAMPLE_RATE),
    "-ac",
    String(CHANNEL),
    "-b:a",
    String(BITRATE),
    outputPath,
  ];

  const result = spawnSync("ffmpeg", args, { stdio: "pipe", encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(
      `ffmpeg failed: ${result.stderr || result.stdout || "unknown error"}`,
    );
  }
}

function finalizeHeadingAudio(outPath, audioBuffer) {
  const tempRawPath = `${outPath}.tmp.base.mp3`;
  const tempFinalPath = `${outPath}.tmp.final.mp3`;

  fs.writeFileSync(tempRawPath, audioBuffer);
  addSilenceWithFfmpeg(
    tempRawPath,
    tempFinalPath,
    HEADING_PRE_SILENCE_SEC,
    HEADING_POST_SILENCE_SEC,
  );
  fs.renameSync(tempFinalPath, outPath);
  fs.unlinkSync(tempRawPath);
}

function finalizeParagraphAudio(outPath, audioBuffer) {
  fs.writeFileSync(outPath, audioBuffer);
}

function validateConstants() {
  if (!API_KEY || API_KEY === "YOUR_MINIMAX_API_KEY") {
    throw new Error("Please set API_KEY constant before running");
  }
  if (!GROUP_ID || GROUP_ID === "YOUR_MINIMAX_GROUP_ID") {
    throw new Error("Please set GROUP_ID constant before running");
  }
}

async function main() {
  validateConstants();
  requireFfmpeg();

  const args = parseArgs(process.argv);
  const markdownPath = path.resolve(args.m);
  const outputDir = path.resolve(args.d);

  if (!fs.existsSync(markdownPath)) {
    throw new Error(`Markdown file not found: ${markdownPath}`);
  }

  ensureDir(outputDir);

  const markdown = fs.readFileSync(markdownPath, "utf8");
  const allChunks = splitMarkdownIntoChunks(markdown);

  if (allChunks.length === 0) {
    throw new Error("No headings or paragraphs found to synthesize");
  }

  const selectedChunks = args.n > 0 ? allChunks.slice(0, args.n) : allChunks;

  console.log(`Markdown: ${markdownPath}`);
  console.log(`Output dir: ${outputDir}`);
  console.log(`Total parsed chunks: ${allChunks.length}`);
  console.log(`Chunks to synthesize (respecting -n): ${selectedChunks.length}`);
  console.log(`Model: ${MODEL}`);
  console.log(`Voice: ${VOICE_ID}`);

  if (args.dryRun) {
    console.log("\n=== Dry run: chunk list ===");
    selectedChunks.forEach((chunk, idx) => {
      const len = chunk.text.length;
      const preview = chunk.text.slice(0, 80).replace(/\s+/g, " ");
      console.log(
        `#${idx + 1} type=${chunk.type} len=${len} preview=${preview}`,
      );
    });
    console.log("=== End of dry run ===");
    return;
  }

  for (let i = 0; i < selectedChunks.length; i++) {
    const chunk = selectedChunks[i];
    const fileIndex = i + 1;
    const filename = chunkFilename(fileIndex);
    const outPath = path.join(outputDir, filename);

    console.log(
      `\n[${fileIndex}/${selectedChunks.length}] type=${chunk.type} -> ${filename}`,
    );
    console.log(
      `[${fileIndex}/${selectedChunks.length}] len=${chunk.text.length} preview=${chunk.text.slice(0, 120)}`,
    );

    const { audioBuffer } = await synthesizeChunk(
      chunk.text,
      fileIndex,
      selectedChunks.length,
    );

    if (chunk.type === "heading") {
      finalizeHeadingAudio(outPath, audioBuffer);
    } else {
      finalizeParagraphAudio(outPath, audioBuffer);
    }

    const stat = fs.statSync(outPath);
    console.log(
      `[${fileIndex}/${selectedChunks.length}] saved ${outPath} (${stat.size} bytes)`,
    );
  }

  console.log("\nDone.");
}

main().catch((err) => {
  console.error(`Fatal error: ${err.message}`);
  process.exit(1);
});
