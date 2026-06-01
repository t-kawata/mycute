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
  let minX = Infinity,
    maxX = -Infinity,
    minY = Infinity,
    maxY = -Infinity;
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

function ageColor(age) {
  const RED = 0xff2b2b;
  const BLUE = 0x3355ff;
  const t = Math.min(age / STATE.adultAge, 1);
  return lerpColor(RED, BLUE, t);
}

// ============================================================
// グローバル状態
// ============================================================
const STATE = {
  ws: null,
  app: null,
  nodeLayer: null,
  villageLayer: null,
  nodes: new Map(), // nodeId -> { graphics, rawX, rawY, targetX, targetY, x, y, ... }
  villages: new Map(),
  maxTicks: 200,
  adultAge: 20,
  currentTick: 0,
  population: 0,
  childCount: 0,
  births: 0,
  deaths: 0,
  villageCount: 0,
  benevolenceP50: 0,
  chiefdomP50: 0,
  chiefCount: 0,
  chiefs: {},
  running: false,
  reconnecting: false,
  radiusScale: 1.0,
  renderInterval: 1,
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
  const sorted = points
    .slice()
    .sort((a, b) => (a.x !== b.x ? a.x - b.x : a.y - b.y));
  const lower = [];
  for (const p of sorted) {
    while (
      lower.length >= 2 &&
      cross(lower[lower.length - 2], lower[lower.length - 1], p) <= 0
    )
      lower.pop();
    lower.push(p);
  }
  const upper = [];
  for (let i = sorted.length - 1; i >= 0; i--) {
    const p = sorted[i];
    while (
      upper.length >= 2 &&
      cross(upper[upper.length - 2], upper[upper.length - 1], p) <= 0
    )
      upper.pop();
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
    background: "#efefef",
    antialias: true,
  });

  document.getElementById("app").appendChild(app.canvas);

  // レイヤー構成 (bottom→top): ノード → 村
  STATE.villageLayer = new PIXI.Container();
  STATE.nodeLayer = new PIXI.Container();
  STATE.nodeLayer.sortableChildren = true;
  app.stage.addChild(STATE.nodeLayer);
  app.stage.addChild(STATE.villageLayer);

  STATE.app = app;
}

// ============================================================
// WebSocket 接続
// ============================================================
function connectWebSocket() {
  if (STATE.reconnecting) return;
  const protocol = location.protocol === "https:" ? "wss:" : "ws:";
  const wsUrl = `${protocol}//${location.host}/ws`;

  STATE.ws = new WebSocket(wsUrl);

  STATE.ws.onopen = () => {
    STATE.reconnecting = false;
    document.getElementById("connectionStatus").textContent = "接続済み";
    document.getElementById("connectionStatus").style.color = "#4ade80";
    // 接続時にフロントエンド設定をバックエンドに同期（フロントエンド正典）
    syncSettingsToBackend();
  };

  STATE.ws.onmessage = (event) => {
    try {
      const msg = JSON.parse(event.data);
      handleEvent(msg);
    } catch (e) {
      console.error("イベント解析エラー:", e);
    }
  };

  STATE.ws.onclose = () => {
    document.getElementById("connectionStatus").textContent = "切断";
    document.getElementById("connectionStatus").style.color = "#f87171";
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
    case "Lifecycle":
      if (subKind === "NodeCreated") {
        onNodeCreated(payload, tick);
      }
      break;
    case "System":
      if (subKind === "ClockAdvanced") {
        onTickMetrics(payload);
      }
      break;
    case "Village":
      if (subKind === "TickCompleted") {
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
  const radius = nodeCount * STATE.radiusScale; // 半径 = 総再帰ノード数 × 倍率 [px]

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

  // 新規ノード: データのみ保存、Graphics は onTickMetrics の描画フェーズで生成
  const screen = rawToScreen(raw.x, raw.y, STATE.curTransform);
  STATE.nodes.set(nodeId, {
    rawX: raw.x,
    rawY: raw.y,
    targetX: screen.x,
    targetY: screen.y,
    x: screen.x,
    y: screen.y,
    radius,
    color: ageColor(0),
    age: 0,
    is_child,
    benevolence,
    node_count: nodeCount,
  });
  // graphics と label は undefined → onTickMetrics 描画フェーズで生成される
}

function onTickMetrics(payload) {
  STATE.benevolenceP50 = payload.benevolence_p50 || 0;

  // 描画間引き: renderInterval の倍数 tick のみフル描画
  const tick = payload.tick || 0;
  const ri = STATE.renderInterval || 1;
  const shouldRender = tick === 0 || ri <= 1 || tick % ri === 0;

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

  // 描画間引き: 非描画 tick では全描画処理をスキップ（死亡/属性/村/graphics）
  if (shouldRender) {
    STATE.currentTick = tick;
    // フェーズ0: 死亡ノードを削除（nodesData に含まれないノードは死亡）
    for (const [nodeId, node] of STATE.nodes) {
      if (!(nodeId in nodesData)) {
        if (node.graphics) {
          STATE.nodeLayer.removeChild(node.graphics);
          node.graphics.destroy();
          STATE.nodeLayer.removeChild(node.label);
          node.label.destroy();
        }
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
          node.radius = serverNode.node_count * STATE.radiusScale;
          if (node.label) {
            node.label.text = String(serverNode.node_count);
          }
        }
        if (serverNode.is_child != null) {
          node.is_child = serverNode.is_child;
        }
        if (serverNode.chiefdom_score != null) {
          node.chiefdom_score = serverNode.chiefdom_score;
        }
      }
    }

    // 首長マップをリセットしてから設定
    STATE.chiefs = {};
    if (payload.village_chiefs) {
      for (const [, chiefPid] of Object.entries(payload.village_chiefs)) {
        STATE.chiefs[chiefPid] = true;
      }
    }

    // 首長性中央値を集計
    const chiefdomScores = [];
    for (const [, node] of STATE.nodes) {
      if (node.chiefdom_score != null) {
        chiefdomScores.push(node.chiefdom_score);
      }
    }
    if (chiefdomScores.length > 0) {
      chiefdomScores.sort((a, b) => a - b);
      const mid = Math.floor(chiefdomScores.length / 2);
      STATE.chiefdomP50 = chiefdomScores.length % 2 === 0
        ? (chiefdomScores[mid - 1] + chiefdomScores[mid]) / 2
        : chiefdomScores[mid];
    } else {
      STATE.chiefdomP50 = 0;
    }
    STATE.chiefCount = Object.keys(payload.village_chiefs || {}).length || 0;

    STATE._lastRenderTick = STATE.currentTick;
    // フェーズ1.5: 未作成 Graphics ノードを生成（onNodeCreated でデータのみ保存されたノード）
    for (const [nodeId, node] of STATE.nodes) {
      if (node.graphics) continue;
      const g = new PIXI.Graphics();
      g.circle(0, 0, node.radius);
      g.fill(node.color);
      g.stroke({ width: 1, color: 0xffffff });
      g.x = node.x;
      g.y = node.y;
      STATE.nodeLayer.addChild(g);
      const label = new PIXI.Text({
        text: String(node.node_count || 0),
        style: { fontSize: 10, fill: "#ffffff", fontFamily: "monospace", fontWeight: "bold" },
      });
      label.anchor.set(0.5, 0.5);
      label.x = node.x;
      label.y = node.y;
      STATE.nodeLayer.addChild(label);
      node.graphics = g;
      node.label = label;
    }

    // フェーズ2: 全ノードの raw 座標からバウンディングボックスを計算
    const screenW = STATE.app.screen.width;
    const screenH = STATE.app.screen.height;
    STATE.tgtTransform = computeTransform(STATE.nodes, screenW, screenH);

    // フェーズ3: transform を全ノードに適用して即時画面座標に反映
    for (const [nodeId, node] of STATE.nodes) {
      const screen = rawToScreen(node.rawX, node.rawY, STATE.tgtTransform);
      node.graphics.x = screen.x;
      node.graphics.y = screen.y;
      node.label.x = screen.x;
      node.label.y = screen.y;

      // 色を再描画: 首長は黒、それ以外は年齢色
      const fillColor = STATE.chiefs[nodeId]
        ? 0x000000
        : ageColor(node.age);
      const strokeColor = 0xffffff;
      node.graphics.clear();
      node.graphics.circle(0, 0, node.radius);
      node.graphics.fill(fillColor);
      node.graphics.stroke({ width: 1, color: strokeColor });

      // 首長の zIndex を上げて常に最前面に表示
      const chiefZ = STATE.chiefs[nodeId] ? 1 : 0;
      node.graphics.zIndex = chiefZ;
      node.label.zIndex = chiefZ;
    }

    // 村外縁を描画（tick 受信時にのみ再計算、アニメーションフレームでは再描画しない）
    STATE.villageNodesData = nodesData;
    // curTransform を tgtTransform に即時スナップしてから村描画
    STATE.curTransform = { ...STATE.tgtTransform };
    renderVillages(nodesData, screenW, screenH);
  }

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
  STATE.villageLayer.removeChildren().forEach((c) => c.destroy(true));

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

    // 村の人口ラベル（白色＋強い影）
    const centroidX = points.reduce((s, p) => s + p.x, 0) / points.length;
    const centroidY = points.reduce((s, p) => s + p.y, 0) / points.length;
    const label = new PIXI.Text({
      text: String(points.length),
      style: {
        fontSize: 20,
        fill: "#ffffff",
        fontFamily: "monospace",
        fontWeight: "bold",
        dropShadow: true,
        dropShadowColor: "#000000",
        dropShadowBlur: 6,
        dropShadowDistance: 3,
      },
    });
    label.anchor.set(0.5, 0.5);
    label.x = centroidX;
    label.y = centroidY;
    STATE.villageLayer.addChild(label);
  }
}

// ============================================================
// PixiJS アニメーションループ（tick 受信時のみレンダリング）
// ============================================================
function setupAnimationLoop() {
  // アニメーションループは空 — 描画は onTickMetrics で行う。
  // PixiJS は STATE.app.ticker により自動的に stage を再描画する。
}

// ============================================================
// コントロールパネル
// ============================================================
function setupControls() {
  // adultAge を入力から同期
  const adultAgeInput = document.getElementById("adultAge");
  function syncAdultAge() {
    STATE.adultAge = parseInt(adultAgeInput.value) || 20;
  }
  adultAgeInput.addEventListener("input", syncAdultAge);
  syncAdultAge();

  // 描画間隔（実行中に動的更新）
  const renderIntervalInput = document.getElementById("renderInterval");
  function syncRenderInterval() {
    STATE.renderInterval = parseInt(renderIntervalInput.value) || 1;
  }
  renderIntervalInput.addEventListener("input", syncRenderInterval);
  syncRenderInterval();

  // 移動距離スライダー（実行中に動的更新）
  const movementSlider = document.getElementById("movementDistance");
  const movementVal = document.getElementById("movementDistanceVal");
  movementSlider.addEventListener("input", () => {
    const val = parseFloat(movementSlider.value);
    movementVal.textContent = val.toFixed(3);
    sendCommand("update_param", { movement_distance: val });
  });

  // 首長引力強度スライダー（実行中に動的更新）
  const chiefAttractionSlider = document.getElementById("chiefAttraction");
  const chiefAttractionVal = document.getElementById("chiefAttractionVal");
  if (chiefAttractionSlider) {
    chiefAttractionSlider.addEventListener("input", () => {
      const val = parseFloat(chiefAttractionSlider.value);
      if (chiefAttractionVal) chiefAttractionVal.textContent = val.toFixed(1);
      sendCommand("update_param", { chief_attraction_strength: val });
    });
  }

  // 最小接近距離（実行中に動的更新）
  const minApproachInput = document.getElementById("minApproach");
  if (minApproachInput) {
    minApproachInput.addEventListener("input", () => {
      const val = parseFloat(minApproachInput.value) || 0.01;
      sendCommand("update_param", { min_approach_distance: val });
    });
  }

  // 目標人口スライダー（実行中に動的更新）
  const targetPopulationSlider = document.getElementById("targetPopulation");
  const targetPopulationVal = document.getElementById("targetPopulationVal");
  if (targetPopulationSlider) {
    targetPopulationSlider.addEventListener("input", () => {
      const val = parseInt(targetPopulationSlider.value) || 0;
      if (targetPopulationVal) targetPopulationVal.textContent = val;
      sendCommand("update_param", { target_population: val });
    });
  }

  // 圧力ランプ範囲（実行中に動的更新）
  const rampRangeInput = document.getElementById("pressureRampRange");
  if (rampRangeInput) {
    rampRangeInput.addEventListener("input", () => {
      const val = parseInt(rampRangeInput.value) || 50;
      sendCommand("update_param", { pressure_ramp_range: val });
    });
  }

  // 上昇時定数（実行中に動的更新）
  const rampUpInput = document.getElementById("pressureRampUp");
  if (rampUpInput) {
    rampUpInput.addEventListener("input", () => {
      const val = parseInt(rampUpInput.value) || 10;
      sendCommand("update_param", { pressure_ramp_up_ticks: val });
    });
  }

  // 下降時定数（実行中に動的更新）
  const rampDownInput = document.getElementById("pressureRampDown");
  if (rampDownInput) {
    rampDownInput.addEventListener("input", () => {
      const val = parseInt(rampDownInput.value) || 20;
      sendCommand("update_param", { pressure_ramp_down_ticks: val });
    });
  }

  // 半径倍率スライダー（即時反映）
  const radiusSlider = document.getElementById("radiusScale");
  const radiusVal = document.getElementById("radiusScaleVal");
  radiusSlider.addEventListener("input", () => {
    const val = parseFloat(radiusSlider.value);
    radiusVal.textContent = val.toFixed(1);
    STATE.radiusScale = val;
    // 既存ノードの半径を即時更新
    for (const [nodeId, node] of STATE.nodes) {
      const scaled = (node.node_count || 1) * val;
      node.radius = scaled;
      node.graphics.clear();
      node.graphics.circle(0, 0, scaled);
      node.graphics.fill(STATE.chiefs[nodeId] ? 0x000000 : ageColor(node.age));
      node.graphics.stroke({ width: 1, color: 0xffffff });
    }
  });

  document.getElementById("startBtn").addEventListener("click", () => {
    syncAdultAge();
    sendCommand("start", {
      population_size:
        parseInt(document.getElementById("popSize").value) || 100,
      max_ticks: parseInt(document.getElementById("maxTicks").value) || 200,
      target_village_size:
        parseFloat(document.getElementById("villageSize").value) || 50,
      village_recluster_interval:
        parseInt(document.getElementById("villageReclusterInterval").value) || 1,
      skip_child_search:
        document.getElementById("skipChildSearch").checked,
      reputation_recompute_interval:
        parseInt(document.getElementById("reputationRecomputeInterval").value) || 1,
    });
    setRunning(true);
  });

  document.getElementById("stopBtn").addEventListener("click", () => {
    sendCommand("stop");
    setRunning(false);
  });

  document.getElementById("resetBtn").addEventListener("click", () => {
    sendCommand("reset");
    clearVisualization();
    setRunning(false);
  });
}

function setRunning(running) {
  STATE.running = running;
  document.getElementById("startBtn").disabled = running;
  document.getElementById("stopBtn").disabled = !running;
}

function sendCommand(command, config) {
  const msg = config
    ? JSON.stringify({ command, config })
    : JSON.stringify({ command });
  if (STATE.ws && STATE.ws.readyState === WebSocket.OPEN) {
    STATE.ws.send(msg);
  }
}

// 接続時にフロントエンドの全設定をバックエンドに同期する
// フロントエンドの設定値を正典として、バックエンドをそれに合わせる
function syncSettingsToBackend() {
  const movementSlider = document.getElementById("movementDistance");
  if (movementSlider) {
    const val = parseFloat(movementSlider.value) || 0.001;
    sendCommand("update_param", { movement_distance: val });
  }
  const chiefSlider = document.getElementById("chiefAttraction");
  if (chiefSlider) {
    const val = parseFloat(chiefSlider.value) || 1.0;
    sendCommand("update_param", { chief_attraction_strength: val });
  }
  const minApproachInput = document.getElementById("minApproach");
  if (minApproachInput) {
    const val = parseFloat(minApproachInput.value) || 0.01;
    sendCommand("update_param", { min_approach_distance: val });
  }

  // 目標人口をバックエンドに同期
  const targetPopSlider = document.getElementById("targetPopulation");
  if (targetPopSlider) {
    const val = parseInt(targetPopSlider.value) || 0;
    sendCommand("update_param", { target_population: val });
  }

  // 圧力制御パラメータをバックエンドに同期
  const rampRangeInput = document.getElementById("pressureRampRange");
  if (rampRangeInput) {
    const val = parseInt(rampRangeInput.value) || 50;
    sendCommand("update_param", { pressure_ramp_range: val });
  }
  const rampUpInput = document.getElementById("pressureRampUp");
  if (rampUpInput) {
    const val = parseInt(rampUpInput.value) || 10;
    sendCommand("update_param", { pressure_ramp_up_ticks: val });
  }
  const rampDownInput = document.getElementById("pressureRampDown");
  if (rampDownInput) {
    const val = parseInt(rampDownInput.value) || 20;
    sendCommand("update_param", { pressure_ramp_down_ticks: val });
  }
}

// ============================================================
// 統計パネル
// ============================================================
function updateStatsPanel(metrics) {
  document.getElementById("tickVal").textContent = STATE.currentTick;
  document.getElementById("popVal").textContent = STATE.population;
  document.getElementById("childVal").textContent = STATE.childCount;
  document.getElementById("birthVal").textContent = STATE.births;
  document.getElementById("deathVal").textContent = STATE.deaths;
  document.getElementById("villageVal").textContent = STATE.villageCount;
  document.getElementById("benevolenceP50Val").textContent = (
    STATE.benevolenceP50 || 0
  ).toFixed(3);
  document.getElementById("chiefdomP50Val").textContent = (
    STATE.chiefdomP50 || 0
  ).toFixed(3);
  document.getElementById("chiefCountVal").textContent = (
    STATE.chiefCount || 0
  );

}

// ============================================================
// 可視化クリア
// ============================================================
function clearVisualization() {
  // ノードレイヤーをクリア（グラフィックス＋ラベル、未作成の pending ノードはデータのみ削除）
  for (const [, node] of STATE.nodes) {
    if (node.graphics) {
      STATE.nodeLayer.removeChild(node.graphics);
      node.graphics.destroy();
      STATE.nodeLayer.removeChild(node.label);
      node.label.destroy();
    }
  }
  STATE.nodes.clear();

  // 村レイヤーをクリア
  STATE.villageLayer.removeChildren().forEach((c) => c.destroy(true));
  STATE.villages.clear();

  STATE.curTransform = { scale: 1, offsetX: 0, offsetY: 0 };
  STATE.tgtTransform = { scale: 1, offsetX: 0, offsetY: 0 };
  STATE.villageNodesData = null;
  STATE.currentTick = 0;
  STATE._lastRenderTick = undefined;
  STATE.population = 0;
  STATE.childCount = 0;
  STATE.births = 0;
  STATE.deaths = 0;
  STATE.villageCount = 0;
  STATE.benevolenceP50 = 0;
  STATE.chiefdomP50 = 0;
  STATE.chiefCount = 0;
  STATE.chiefs = {};
  STATE.adultAge = parseInt(document.getElementById("adultAge").value) || 20;

  document.getElementById("tickVal").textContent = "0";
  document.getElementById("popVal").textContent = "0";
  document.getElementById("childVal").textContent = "0";
  document.getElementById("birthVal").textContent = "0";
  document.getElementById("deathVal").textContent = "0";
  document.getElementById("villageVal").textContent = "0";
  document.getElementById("benevolenceP50Val").textContent = "0.000";
  document.getElementById("chiefdomP50Val").textContent = "0.000";
  document.getElementById("chiefCountVal").textContent = "0";
  document.getElementById("densityList").innerHTML = "";
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
