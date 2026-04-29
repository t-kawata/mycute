#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

const sourceDir = path.resolve(__dirname, '../web/public/neco-asovi/large');
const outputDir = path.resolve(__dirname, '../web/public/neco-asovi');

function walkPngFiles(dir) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...walkPngFiles(fullPath));
    } else if (entry.isFile() && entry.name.toLowerCase().endsWith('.png')) {
      files.push(fullPath);
    }
  }

  return files;
}

function identifySize(filePath) {
  const result = execFileSync('identify', ['-format', '%w %h', filePath], {
    encoding: 'utf8',
  }).trim();

  const [width, height] = result.split(/\s+/).map(Number);
  if (!Number.isFinite(width) || !Number.isFinite(height)) {
    throw new Error(`Failed to identify image size: ${filePath}`);
  }

  return { width, height };
}

function resizeImage(inputPath, outputPath, targetWidth) {
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  execFileSync('convert', [
    inputPath,
    '-resize', `${targetWidth}x`,
    outputPath,
  ], { stdio: 'inherit' });
}

function buildOutputPath(inputPath, targetWidth) {
  const relativePath = path.relative(sourceDir, inputPath);
  const parsed = path.parse(relativePath);
  const fileName = `${parsed.name}-${targetWidth}w.png`;
  return path.join(outputDir, parsed.dir, fileName);
}

function main() {
  if (!fs.existsSync(sourceDir)) {
    console.error(`Source directory not found: ${sourceDir}`);
    process.exit(1);
  }

  fs.mkdirSync(outputDir, { recursive: true });

  const pngFiles = walkPngFiles(sourceDir);
  if (pngFiles.length === 0) {
    console.log('No PNG files found.');
    return;
  }

  for (const inputPath of pngFiles) {
    const { width, height } = identifySize(inputPath);
    const targetWidths = width === height ? [256, 512] : [512, 768];

    for (const targetWidth of targetWidths) {
      const outputPath = buildOutputPath(inputPath, targetWidth);
      console.log(`Converting: ${inputPath} -> ${outputPath}`);
      resizeImage(inputPath, outputPath, targetWidth);
    }
  }

  console.log(`Done. Processed ${pngFiles.length} PNG file(s).`);
}

main();
