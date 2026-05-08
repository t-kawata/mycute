#!/usr/bin/env node
const fs = require("fs");
const path = require("path");
const os = require("os");
const { spawnSync } = require("child_process");

function run(cmd, args, options = {}) {
  const r = spawnSync(cmd, args, {
    stdio: "inherit",
    ...options,
  });
  if (r.error) throw r.error;
  if (r.status !== 0) {
    throw new Error(`${cmd} failed with exit code ${r.status}`);
  }
}

function pad(n, width = 4) {
  return String(n).padStart(width, "0");
}

function main() {
  const targetDir = process.argv[2];
  if (!targetDir) {
    console.error("Usage: node concat-mp3.js <target-directory> [output-file]");
    process.exit(1);
  }

  const absDir = path.resolve(targetDir);
  const outputFile = process.argv[3]
    ? path.resolve(process.argv[3])
    : path.join(absDir, "merged.mp3");

  if (!fs.existsSync(absDir) || !fs.statSync(absDir).isDirectory()) {
    throw new Error(`Directory not found: ${absDir}`);
  }

  const files = fs
    .readdirSync(absDir)
    .filter((f) => /^\d+\.mp3$/i.test(f))
    .sort((a, b) => {
      const na = Number(path.parse(a).name);
      const nb = Number(path.parse(b).name);
      return na - nb;
    });

  if (files.length === 0) {
    throw new Error("No numbered mp3 files found (e.g. 0001.mp3, 0002.mp3)");
  }

  const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "concat-mp3-"));
  const wavDir = path.join(tmpRoot, "wav");
  fs.mkdirSync(wavDir);

  const sampleRate = 44100;
  const channels = 2;
  const channelLayout = "stereo";

  const silenceWav = path.join(tmpRoot, "silence.wav");
  const concatList = path.join(tmpRoot, "concat.txt");

  try {
    for (let i = 0; i < files.length; i++) {
      const src = path.join(absDir, files[i]);
      const dst = path.join(wavDir, `${pad(i + 1)}.wav`);

      run("ffmpeg", [
        "-y",
        "-i",
        src,
        "-ar",
        String(sampleRate),
        "-ac",
        String(channels),
        "-c:a",
        "pcm_s16le",
        dst,
      ]);
    }

    run("ffmpeg", [
      "-y",
      "-f",
      "lavfi",
      "-t",
      "0.5",
      "-i",
      `anullsrc=r=${sampleRate}:cl=${channelLayout}`,
      "-c:a",
      "pcm_s16le",
      silenceWav,
    ]);

    const lines = [];
    for (let i = 0; i < files.length; i++) {
      const wavPath = path.join(wavDir, `${pad(i + 1)}.wav`);
      lines.push(`file '${wavPath.replace(/'/g, "'\\''")}'`);
      if (i !== files.length - 1) {
        lines.push(`file '${silenceWav.replace(/'/g, "'\\''")}'`);
      }
    }
    fs.writeFileSync(concatList, lines.join("\n") + "\n");

    run("ffmpeg", [
      "-y",
      "-f",
      "concat",
      "-safe",
      "0",
      "-i",
      concatList,
      "-c:a",
      "libmp3lame",
      "-b:a",
      "192k",
      outputFile,
    ]);

    console.log(`Done: ${outputFile}`);
  } finally {
    fs.rmSync(tmpRoot, { recursive: true, force: true });
  }
}

main();
