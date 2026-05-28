// Darvium シミュレーション可視化 — PixiJS + WebSocket
//
// WebSocket 経由でサーバーから DarviumEvent を受信し、
// 3D→2D 等角投影でワークフローを円形として描画する。

// ============================================================
// 等角投影変換 (3D→2D)
// ============================================================
// 固定回転行列 (X 軸 ~35.26°, Y 軸 45°) で [0,1)^3 空間を 2D に射影。
// 全 3 成分が寄与し、いずれの次元も捨てない。
//
// 2段階方式:
//   1. projectRaw() — 3D→2D の生の投影座標を計算（画面サイズ非依存）
//   2. rawToScreen() — バウンディングボックス基準の動的スケール/オフセットを適用

function projectRaw(x, y, z) {
    return {
        x: x * 0.707 - y * 0.707,
        y: x * 0.408 + y * 0.408 - z * 0.816,
    };
}

function rawToScreen(rawX, rawY, transform) {
    return {
        x: rawX * transform.scale + transform.offsetX,
        y: rawY * transform.scale + transform.offsetY,
    };
}

// 全ノードの raw 座標から、画面にフィットする transform を計算
function computeTransform(nodes, screenW, screenH) {
    if (nodes.size === 0) {
        return { scale: 1, offsetX: screenW / 2, offsetY: screenH / 2 };
    }
    const PAD = 0.06; // 画面端の余白比率
    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
    for (const [, node] of nodes) {
        if (node.rawX < minX) minX = node.rawX;
        if (node.rawX > maxX) maxX = node.rawX;
        if (node.rawY < minY) minY = node.rawY;
        if (node.rawY > maxY) maxY = node.rawY;
    }
    const rangeX = Math.max(maxX - minX, 0.001);
    const rangeY = Math.max(maxY - minY, 0.001);
    const scale = Math.min(
        screenW / (rangeX * (1 + 2 * PAD)),
        screenH / (rangeY * (1 + 2 * PAD)),
    );
    const centerX = (minX + maxX) / 2;
    const centerY = (minY + maxY) / 2;
    return {
        scale,
        offsetX: screenW / 2 - centerX * scale,
        offsetY: screenH / 2 - centerY * scale,
    };
}

// ============================================================
// 多角形面積 (Shoelace formula)
// ============================================================
function polygonArea(points) {
    let area = 0;
    const n = points.length;
    if (n < 3) return 0;
    for (let i = 0; i < n; i++) {
        const j = (i + 1) % n;
        area += points[i].x * points[j].y;
        area -= points[j].x * points[i].y;
    }
    return Math.abs(area) / 2;
}

// ============================================================
// 村密度計算 (面積あたりのノード数)
// ============================================================
function computeVillageDensities(nodesData) {
    if (!nodesData || Object.keys(nodesData).length === 0) return {};
    // village_id → raw投影座標の配列
    const groups = {};
    for (const [nodeId, data] of Object.entries(nodesData)) {
        const vId = data.village_id;
        if (vId === undefined || vId === null) continue;
        const pos = data.position;
        if (!pos || pos.length !== 3) continue;
        const raw = projectRaw(pos[0], pos[1], pos[2]);
        if (!groups[vId]) groups[vId] = [];
        groups[vId].push(raw);
    }
    const densities = {};
    const EPS_AREA = 1e-6;
    for (const [vId, pts] of Object.entries(groups)) {
        const hull = convexHull(pts);
        const area = Math.max(polygonArea(hull), EPS_AREA);
        densities[vId] = pts.length / area;
    }
    return densities;
}
// ============================================================
function lerp(a, b, t) {
    return a + (b - a) * t;
}

function lerpColor(c1, c2, t) {
    const r1 = (c1 >> 16) & 255;
    const g1 = (c1 >> 8) & 255;
    const b1 = c1 & 255;
    const r2 = (c2 >> 16) & 255;
    const g2 = (c2 >> 8) & 255;
    const b2 = c2 & 255;
    const r = Math.round(lerp(r1, r2, t));
    const g = Math.round(lerp(g1, g2, t));
    const b = Math.round(lerp(b1, b2, t));
    return (r << 16) | (g << 8) | b;
}

function ageColor(age, maxTicks) {
    const RED = 0xff2b2b;
    const BLACK = 0x000000;
    const t = Math.min(age / (maxTicks || 200), 1);
    return lerpColor(RED, BLACK, t);
}

// ============================================================
// グローバル状態
// ============================================================
const STATE = {
    ws: null,
    app: null,
    nodeLayer: null,
    villageLayer: null,
    nodes: new Map(),         // nodeId -> { graphics, rawX, rawY, targetX, targetY, x, y, ... }
    villages: new Map(),
    maxTicks: 200,
    currentTick: 0,
    population: 0,
    childCount: 0,
    births: 0,
    deaths: 0,
    villageCount: 0,
    benevolenceP50: 0,
    running: false,
    reconnecting: false,
    // 動的バウンディングボックス transform (lerp 対象)
    curTransform: { scale: 1, offsetX: 0, offsetY: 0 },
    tgtTransform: { scale: 1, offsetX: 0, offsetY: 0 },
    // 村描画用の最新ノードデータ（tick 到着時に保存、毎フレーム参照）
    villageNodesData: null,
};

// ============================================================
// 凸包計算 (Andrew's monotone chain)
// ============================================================
function cross(o, a, b) {
    return (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
}

function convexHull(points) {
    if (points.length < 3) return points.slice();
    const sorted = points.slice().sort((a, b) => a.x !== b.x ? a.x - b.x : a.y - b.y);
    const lower = [];
    for (const p of sorted) {
        while (lower.length >= 2 && cross(lower[lower.length - 2], lower[lower.length - 1], p) <= 0) lower.pop();
        lower.push(p);
    }
    const upper = [];
    for (let i = sorted.length - 1; i >= 0; i--) {
        const p = sorted[i];
        while (upper.length >= 2 && cross(upper[upper.length - 2], upper[upper.length - 1], p) <= 0) upper.pop();
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    return lower.concat(upper);
}

// ============================================================
// 村色生成 (黄金角ベースの HSL)
// ============================================================
function villageColor(id) {
    const GOLDEN_ANGLE = 137.508;
    const hue = (id * GOLDEN_ANGLE) % 360;
    return `hsl(${hue}, 60%, 50%)`;
}

// ============================================================
// PixiJS 初期化
// ============================================================
async function initPixi() {
    const app = new PIXI.Application();

    await app.init({
        resizeTo: window,
        background: '#efefef',
        antialias: true,
    });

    document.getElementById('app').appendChild(app.canvas);

    // レイヤー構成 (bottom→top): 村 → ノード
    STATE.villageLayer = new PIXI.Container();
    STATE.nodeLayer = new PIXI.Container();
    app.stage.addChild(STATE.villageLayer);
    app.stage.addChild(STATE.nodeLayer);

    STATE.app = app;
}

// ============================================================
// WebSocket 接続
// ============================================================
function connectWebSocket() {
    if (STATE.reconnecting) return;
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${location.host}/ws`;

    STATE.ws = new WebSocket(wsUrl);

    STATE.ws.onopen = () => {
        STATE.reconnecting = false;
        document.getElementById('connectionStatus').textContent = '接続済み';
        document.getElementById('connectionStatus').style.color = '#4ade80';
    };

    STATE.ws.onmessage = (event) => {
        try {
            const msg = JSON.parse(event.data);
            handleEvent(msg);
        } catch (e) {
            console.error('イベント解析エラー:', e);
        }
    };

    STATE.ws.onclose = () => {
        document.getElementById('connectionStatus').textContent = '切断';
        document.getElementById('connectionStatus').style.color = '#f87171';
        STATE.reconnecting = true;
        setTimeout(() => {
            STATE.reconnecting = false;
            connectWebSocket();
        }, 2000);
    };

    STATE.ws.onerror = () => {
        STATE.ws.close();
    };
}

// ============================================================
// イベントハンドリング
// ============================================================
function handleEvent(event) {
    const kind = event.kind ? Object.keys(event.kind)[0] : null;
    if (!kind) return;

    const subKind = event.kind[kind];
    const payload = event.payload || {};
    const tick = event.metadata?.clock || 0;

    switch (kind) {
        case 'Lifecycle':
            if (subKind === 'NodeCreated') {
                onNodeCreated(payload, tick);
            }
            break;
        case 'System':
            if (subKind === 'ClockAdvanced') {
                onTickMetrics(payload);
            }
            break;
        case 'Village':
            if (subKind === 'TickCompleted') {
                onVillageAssign(payload);
            }
            break;
    }
}

function onNodeCreated(payload, tick) {
    const nodeId = payload.node_id;
    if (nodeId === undefined || nodeId === null) return;

    const pos = payload.position || [0.5, 0.5, 0.5];
    const benevolence = payload.benevolence || 0.5;
    const nodeCount = payload.node_count || 0;
    const is_child = payload.is_child === true;
    const radius = 3 + Math.min(nodeCount, 20) * 0.45; // ノード数ベース (3-12px)

    // raw 投影座標を保存
    const raw = projectRaw(pos[0], pos[1], pos[2]);

    // 既存ノードの位置更新
    if (STATE.nodes.has(nodeId)) {
        const node = STATE.nodes.get(nodeId);
        node.rawX = raw.x;
        node.rawY = raw.y;
        node.is_child = is_child;
        const screen = rawToScreen(raw.x, raw.y, STATE.curTransform);
        node.targetX = screen.x;
        node.targetY = screen.y;
        return;
    }

    // 画面座標に変換して新規ノード描画
    const screen = rawToScreen(raw.x, raw.y, STATE.curTransform);
    const g = new PIXI.Graphics();
    const color = ageColor(0, STATE.maxTicks);
    const strokeColor = is_child ? 0xffd700 : 0xffffff;
    g.circle(0, 0, radius);
    g.fill(color);
    g.stroke({ width: 1, color: strokeColor });
    g.x = screen.x;
    g.y = screen.y;
    STATE.nodeLayer.addChild(g);

    STATE.nodes.set(nodeId, {
        graphics: g,
        rawX: raw.x,
        rawY: raw.y,
        targetX: screen.x,
        targetY: screen.y,
        x: screen.x,
        y: screen.y,
        radius,
        color,
        age: 0,
        is_child,
        benevolence,
        node_count: nodeCount,
    });
}

function onTickMetrics(payload) {
    STATE.currentTick = payload.tick || 0;
    STATE.benevolenceP50 = payload.benevolence_p50 || 0;

    const nodesData = payload.nodes || {};

    // 人口・子供数: 生存ノードデータから直接集計
    STATE.population = Object.keys(nodesData).length;
    STATE.childCount = payload.child_count || 0;
    // 村数: 生存ノードの village_id から直接カウント（サーバーの village_count は不正確な場合がある）
    const seenVillages = new Set();
    for (const [, data] of Object.entries(nodesData)) {
        if (data.village_id !== undefined && data.village_id !== null) {
            seenVillages.add(data.village_id);
        }
    }
    STATE.villageCount = seenVillages.size;
    // 出生・死亡は累積（サーバーからは各tickの値が送られる）
    STATE.births = (STATE.births || 0) + (payload.births || 0);
    STATE.deaths = (STATE.deaths || 0) + (payload.deaths || 0);

    // フェーズ0: 死亡ノードを削除（nodesData に含まれないノードは死亡）
    for (const [nodeId, node] of STATE.nodes) {
        if (!(nodeId in nodesData)) {
            STATE.nodeLayer.removeChild(node.graphics);
            node.graphics.destroy();
            STATE.nodes.delete(nodeId);
        }
    }

    // フェーズ1: 生存ノードの raw 投影座標と属性を更新
    for (const [nodeId, node] of STATE.nodes) {
        node.age = (node.age || 0) + 1;

        const serverNode = nodesData[nodeId];
        if (serverNode) {
            const pos = serverNode.position;
            if (pos && pos.length === 3) {
                const raw = projectRaw(pos[0], pos[1], pos[2]);
                node.rawX = raw.x;
                node.rawY = raw.y;
            }
            if (serverNode.benevolence != null) {
                node.benevolence = serverNode.benevolence;
            }
            if (serverNode.node_count != null) {
                node.node_count = serverNode.node_count;
                node.radius = 3 + Math.min(serverNode.node_count, 20) * 0.45;
            }
            if (serverNode.is_child != null) {
                node.is_child = serverNode.is_child;
            }
        }
    }

    // フェーズ2: 全ノードの raw 座標からバウンディングボックスを計算
    const screenW = STATE.app.screen.width;
    const screenH = STATE.app.screen.height;
    STATE.tgtTransform = computeTransform(STATE.nodes, screenW, screenH);

    // フェーズ3: transform を全ノードに適用して target 画面座標を設定
    for (const [, node] of STATE.nodes) {
        const screen = rawToScreen(node.rawX, node.rawY, STATE.tgtTransform);
        node.targetX = screen.x;
        node.targetY = screen.y;

        // 色を再描画
        const fillColor = ageColor(node.age, STATE.maxTicks);
        const strokeColor = node.is_child ? 0xffd700 : 0xffffff; // 子供=金色, 成人=白
        node.graphics.clear();
        node.graphics.circle(0, 0, node.radius);
        node.graphics.fill(fillColor);
        node.graphics.stroke({ width: 1, color: strokeColor });
    }

    // 村外縁を描画（データをキャッシュし、アニメーションループで毎フレーム描画）
    STATE.villageNodesData = nodesData;

    updateStatsPanel(payload);
}

function onVillageAssign(payload) {
    // payload.assignments は { village_id: [member_node_ids] } の形式
    // 将来拡張用
}

// ============================================================
// 村外縁レンダリング
// ============================================================
function renderVillages(nodesData, screenW, screenH) {
    // 村レイヤーを全クリア
    STATE.villageLayer.removeChildren().forEach(c => c.destroy(true));

    if (!nodesData || Object.keys(nodesData).length === 0) return;

    // village_id -> [{x, y}] にグループ化（raw → screen は curTransform で変換）
    const villagePoints = {};
    const t = STATE.curTransform;
    for (const [nodeId, data] of Object.entries(nodesData)) {
        const vId = data.village_id;
        if (vId === undefined || vId === null) continue;
        const pos = data.position;
        if (!pos || pos.length !== 3) continue;
        const raw = projectRaw(pos[0], pos[1], pos[2]);
        const screen = rawToScreen(raw.x, raw.y, t);
        if (!villagePoints[vId]) {
            villagePoints[vId] = [];
        }
        villagePoints[vId].push({ x: screen.x, y: screen.y });
    }

    for (const [vId, points] of Object.entries(villagePoints)) {
        if (points.length < 2) continue;

        const color = villageColor(parseInt(vId) || 0);
        const g = new PIXI.Graphics();

        if (points.length === 2) {
            g.moveTo(points[0].x, points[0].y);
            g.lineTo(points[1].x, points[1].y);
            g.stroke({ width: 2, color: color });
        } else {
            const hull = convexHull(points);
            if (hull.length < 3) {
                g.moveTo(hull[0].x, hull[0].y);
                g.lineTo(hull[1].x, hull[1].y);
                g.stroke({ width: 2, color: color });
            } else {
                g.moveTo(hull[0].x, hull[0].y);
                for (let i = 1; i < hull.length; i++) {
                    g.lineTo(hull[i].x, hull[i].y);
                }
                g.closePath();
                g.fill({ color: color, alpha: 0.15 });
                g.stroke({ width: 1.5, color: color, alpha: 0.6 });
            }
        }

        STATE.villageLayer.addChild(g);
    }
}

// ============================================================
// PixiJS アニメーションループ (スムーズ移動)
// ============================================================
function setupAnimationLoop() {
    STATE.app.ticker.add(() => {
        const dt = Math.min(STATE.app.ticker.elapsedMS / 16.667, 3);
        const ease = 0.08 * dt;
        const te = 0.04 * dt; // 変換パラメータの補間レート（よりゆっくり）

        // curTransform → tgtTransform を補間
        const c = STATE.curTransform;
        const t = STATE.tgtTransform;
        c.scale += (t.scale - c.scale) * te;
        c.offsetX += (t.offsetX - c.offsetX) * te;
        c.offsetY += (t.offsetY - c.offsetY) * te;

        for (const [, node] of STATE.nodes) {
            // raw 座標に curTransform を適用して screen 座標を計算
            const screen = rawToScreen(node.rawX, node.rawY, c);
            node.graphics.x += (screen.x - node.graphics.x) * ease;
            node.graphics.y += (screen.y - node.graphics.y) * ease;
        }

        // 村外縁も curTransform に追随して毎フレーム再描画
        if (STATE.villageNodesData) {
            renderVillages(
                STATE.villageNodesData,
                STATE.app.screen.width,
                STATE.app.screen.height,
            );
        }
    });
}

// ============================================================
// コントロールパネル
// ============================================================
function setupControls() {
    document.getElementById('startBtn').addEventListener('click', () => {
        sendCommand('start', {
            population_size: parseInt(document.getElementById('popSize').value) || 100,
            max_ticks: parseInt(document.getElementById('maxTicks').value) || 200,
            target_village_size: parseFloat(document.getElementById('villageSize').value) || 50,
        });
        setRunning(true);
    });

    document.getElementById('stopBtn').addEventListener('click', () => {
        sendCommand('stop');
        setRunning(false);
    });

    document.getElementById('resetBtn').addEventListener('click', () => {
        sendCommand('reset');
        clearVisualization();
        setRunning(false);
    });
}

function setRunning(running) {
    STATE.running = running;
    document.getElementById('startBtn').disabled = running;
    document.getElementById('stopBtn').disabled = !running;
}

function sendCommand(command, config) {
    const msg = config
        ? JSON.stringify({ command, config })
        : JSON.stringify({ command });
    if (STATE.ws && STATE.ws.readyState === WebSocket.OPEN) {
        STATE.ws.send(msg);
    }
}

// ============================================================
// 統計パネル
// ============================================================
function updateStatsPanel(metrics) {
    document.getElementById('tickVal').textContent = STATE.currentTick;
    document.getElementById('popVal').textContent = STATE.population;
    document.getElementById('childVal').textContent = STATE.childCount;
    document.getElementById('birthVal').textContent = STATE.births;
    document.getElementById('deathVal').textContent = STATE.deaths;
    document.getElementById('villageVal').textContent = STATE.villageCount;
    document.getElementById('benevolenceP50Val').textContent =
        (STATE.benevolenceP50 || 0).toFixed(3);

    // 村密度一覧（面積あたりノード数、密度順 降順、上位10村）
    const densities = computeVillageDensities(metrics.nodes || {});
    const densityList = document.getElementById('densityList');
    const densityEntries = Object.entries(densities);
    if (densityEntries.length > 0) {
        densityList.innerHTML = '<div style="color:#888;margin-bottom:2px;">密度 (人/area)</div>' +
            densityEntries
            .sort((a, b) => b[1] - a[1])
            .slice(0, 10)
            .map(([vId, d]) =>
                `<div class="density-row"><span class="d-label">#${vId}</span><span class="d-value">${d.toFixed(1)}</span></div>`
            ).join('');
    } else {
        densityList.innerHTML = '';
    }
}

// ============================================================
// 可視化クリア
// ============================================================
function clearVisualization() {
    // ノードレイヤーをクリア
    for (const [, node] of STATE.nodes) {
        STATE.nodeLayer.removeChild(node.graphics);
        node.graphics.destroy();
    }
    STATE.nodes.clear();

    // 村レイヤーをクリア
    STATE.villageLayer.removeChildren().forEach(c => c.destroy(true));
    STATE.villages.clear();

    STATE.curTransform = { scale: 1, offsetX: 0, offsetY: 0 };
    STATE.tgtTransform = { scale: 1, offsetX: 0, offsetY: 0 };
    STATE.villageNodesData = null;
    STATE.currentTick = 0;
    STATE.population = 0;
    STATE.childCount = 0;
    STATE.births = 0;
    STATE.deaths = 0;
    STATE.villageCount = 0;
    STATE.benevolenceP50 = 0;

    document.getElementById('tickVal').textContent = '0';
    document.getElementById('popVal').textContent = '0';
    document.getElementById('childVal').textContent = '0';
    document.getElementById('birthVal').textContent = '0';
    document.getElementById('deathVal').textContent = '0';
    document.getElementById('villageVal').textContent = '0';
    document.getElementById('benevolenceP50Val').textContent = '0.000';
    document.getElementById('densityList').innerHTML = '';
}

// ============================================================
// エントリポイント
// ============================================================
(async () => {
    await initPixi();
    setupAnimationLoop();
    setupControls();
    connectWebSocket();
})();
