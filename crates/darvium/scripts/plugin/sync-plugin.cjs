#!/usr/bin/env node
/**
 * sync-plugin.js — Sync plugin files from marketplace to install cache
 *
 * `claude plugin install` doesn't reliably copy newly added files
 * (commands, scripts, skills) to the install cache. This script
 * reads installed_plugins.json, finds all darvium plugin install
 * paths, and explicitly syncs the required directories from the
 * marketplace directory.
 *
 * Usage: node scripts/plugin/sync-plugin.js
 *
 * Platform: Mac / Windows (Node.js only, no shell dependencies)
 */
const fs = require('fs');
const path = require('path');

const HOME = require('os').homedir();
const INSTALLED_PATH = path.join(HOME, '.claude', 'plugins', 'installed_plugins.json');
const MARKETPLACE_DIR = path.join(HOME, '.claude', 'plugins', 'marketplaces', 'darvium-marketplace');

// 同期対象ディレクトリ — plugin install が確実にコピーしないもの
const SYNC_DIRS = ['commands', 'scripts', 'skills'];

function syncDir(src, dst) {
	if (!fs.existsSync(src)) return;
	if (!fs.existsSync(dst)) fs.mkdirSync(dst, { recursive: true });

	for (const entry of fs.readdirSync(src)) {
		const s = path.join(src, entry);
		const d = path.join(dst, entry);
		const stat = fs.statSync(s);

		if (stat.isDirectory()) {
			syncDir(s, d);
		} else {
			fs.copyFileSync(s, d);
		}
	}
}

function main() {
	if (!fs.existsSync(INSTALLED_PATH)) {
		console.error('[sync-plugin] Error: installed_plugins.json not found at ' + INSTALLED_PATH);
		process.exit(1);
	}

	const installed = JSON.parse(fs.readFileSync(INSTALLED_PATH, 'utf8'));
	const entries = installed.plugins['darvium@darvium-marketplace'];

	if (!entries || entries.length === 0) {
		console.error('[sync-plugin] Error: darvium@darvium-marketplace not found in installed_plugins.json');
		process.exit(1);
	}

	if (!fs.existsSync(MARKETPLACE_DIR)) {
		console.error('[sync-plugin] Error: Marketplace directory not found at ' + MARKETPLACE_DIR);
		process.exit(1);
	}

	for (const entry of entries) {
		const ip = entry.installPath;
		console.log('[sync-plugin] Syncing to ' + ip);

		for (const dir of SYNC_DIRS) {
			const src = path.join(MARKETPLACE_DIR, dir);
			const dst = path.join(ip, dir);
			syncDir(src, dst);
			console.log('[sync-plugin]   Synced ' + dir);
		}
	}

	console.log('[sync-plugin] Done.');
}

main();
