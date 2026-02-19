<template>
  <div :class="['animation-container', IS_TAURI_DESKTOP ? 'is-tauri' : '']" ref="containerRef">
    <canvas ref="canvasRef"></canvas>
    <div
      v-for="badge in renderedBadges"
      :key="badge.id"
      class="visual-badge"
      :style="getBadgeStyle(badge)"
    >
      <span class="badge-content">{{ badge.content }}</span>
    </div>
  </div>
  <div
    ref="badgeWindowRef"
    :class="['__harunohi-badge-window', isBadgeWindowOpen ? '__harunohi-badge-window-visible' : '']"
  >
    <div class="__harunohi-badge-window-wrap"></div>
  </div>
  <BadgeContentDialog v-model="isBadgeContentDialogOpen" :badge="currentBadge" :usrBadges="currentUsrBadges" />
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, type Ref } from "vue";
import planck from "planck";
import BadgeContentDialog from 'src/components/dialogs/BadgeContentDialog.vue';
import { type Badge, type UsrBadge } from "src/models/main";
import { useMainStore } from 'src/stores/main-store'
import { isTauriDesktop } from "src/utils/some"
import TAB from 'src/enums/TAB'

defineOptions({ inheritAttrs: false });

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

const CONFIG = {
  badgeRadiusFrom: 15,
  badgeRadiusTo: 40,
  restitution: 0.8,
  gravity: 100,
  wallThickness: 60,
  friction: 0.2,
  frictionAir: 0.01,
  mouseStiffness: 0.2,
  clickThresholdMs: 200,
  resizeDebounceMs: 250,
} as const;

const mainStore = useMainStore()
const IS_TAURI_DESKTOP = isTauriDesktop()

const PTM = 30;
const BALL_COLORS = ["#97e0e9", "#f099c3", "#c1a4f8", "#eced87"];

const pxToMeters = (px: number): number => px / PTM;
const metersToPx = (m: number): number => m * PTM;

interface RenderedBadge {
  readonly id: number;
  readonly x: number;
  readonly y: number;
  readonly angle: number;
  readonly radius: number;
  readonly color: string;
  readonly content: string;
  readonly badge: Badge;
}

interface CustomBodyData {
  readonly id: number;
  readonly initialRadius: number;
  readonly content: string;
  readonly color: string;
  readonly badge: Badge;
  direction: -1 | 1;
  velocity: number;
}

interface PhysicsWorld {
  world: planck.World;
  walls: { ground: planck.Body; left: planck.Body; right: planck.Body; ceiling: planck.Body };
  animationFrameId: number | null;
  startTime: number;
}

const props = defineProps<{ badges: Badge[]; usrBadges: UsrBadge[]; usrID: number }>();
const emit = defineEmits<{ "click-badge": [badge: Badge, usrBadges: UsrBadge[]] }>();

const containerRef = ref<HTMLElement | null>(null);
const badgeWindowRef = ref<HTMLElement | null>(null);
const canvasRef = ref<HTMLCanvasElement | null>(null);
const renderedBadges = ref<RenderedBadge[]>([]);
const isBadgeWindowOpen = ref(false);
const isBadgeContentDialogOpen = ref(false);
const currentBadge = ref({} as Badge);
const currentUsrBadges = ref<UsrBadge[]>([]);

let physicsWorld: PhysicsWorld | null = null;
let potentialClickBody: planck.Body | null = null;
let mouseDownTimestamp = 0;
let mouseDownPosition = { x: 0, y: 0 };
let draggingBody: planck.Body | null = null;
let dragOffset: { x: number; y: number } | null = null;
let dragHistory: { x: number; y: number; t: number }[] = [];

let resizeTimeout: number;

const handleResize = () => {
  clearTimeout(resizeTimeout);
  resizeTimeout = window.setTimeout(() => {
    if (!containerRef.value || !canvasRef.value) return;
    if (physicsWorld) {
      cleanupPhysics(physicsWorld);
    }
    physicsWorld = initializePhysics(containerRef.value, canvasRef.value, props.badges);
  }, CONFIG.resizeDebounceMs);
};

const getBadgeStyle = (badge: RenderedBadge): Record<string, string> => ({
  transform: `translate(-50%, -50%) translate3d(${badge.x}px, ${badge.y}px, 0) rotate(${badge.angle}rad)`,
  width: `${badge.radius * 2}px`,
  height: `${badge.radius * 2}px`,
  backgroundColor: badge.color ?? "#cccccc",
});

const getMouseCoords = (event: MouseEvent | Touch): { x: number; y: number } => {
  const rect = containerRef.value!.getBoundingClientRect();
  if ("clientX" in event) {
    return {
      x: event.clientX - rect.left,
      y: event.clientY - rect.top,
    };
  } else {
    const touch = event as Touch;
    return {
      x: touch.clientX - rect.left,
      y: touch.clientY - rect.top,
    };
  }
};

const findBodyAtPos = (mouseX: number, mouseY: number): planck.Body | null => {
  if (!physicsWorld) return null;
  const mouseXM = pxToMeters(mouseX);
  const mouseYM = pxToMeters(mouseY);
  for (let body = physicsWorld.world.getBodyList(); body; body = body.getNext()) {
    const userData = body.getUserData() as CustomBodyData | null;
    if (userData && body.isDynamic()) {
      const position = body.getPosition();
      const radiusM = pxToMeters(userData.initialRadius);
      const dx = position.x - mouseXM;
      const dy = position.y - mouseYM;
      const distanceSq = dx * dx + dy * dy;
      if (distanceSq <= radiusM * radiusM) {
        return body;
      }
    }
  }
  return null;
};

const handleMouseDown = (event: MouseEvent | Touch) => {
  if (!containerRef.value || !physicsWorld) return;
  const { x, y } = getMouseCoords(event);
  mouseDownPosition = { x, y };
  mouseDownTimestamp = Date.now();
  potentialClickBody = findBodyAtPos(x, y);

  draggingBody = findBodyAtPos(x, y);
  dragHistory = [];
  if (draggingBody) {
    dragHistory.push({ x, y, t: Date.now() });
    const pos = draggingBody.getPosition();
    dragOffset = {
      x: pxToMeters(x) - pos.x,
      y: pxToMeters(y) - pos.y,
    };
  }
};

const handleMouseUp = (event: MouseEvent | Touch) => {
  if (!containerRef.value || !potentialClickBody) {
    potentialClickBody = null;
    draggingBody = null;
    dragOffset = null;
    dragHistory = [];
    return;
  }
  const { x, y } = getMouseCoords(event);
  const elapsedTime = Date.now() - mouseDownTimestamp;
  const dx = x - mouseDownPosition.x;
  const dy = y - mouseDownPosition.y;
  const movementSq = dx * dx + dy * dy;

  // フリック慣性処理 (型ガード付きで未定義排除)
  if (draggingBody && dragHistory.length >= 2) {
    const h1 = dragHistory[dragHistory.length - 1];
    const h2 = dragHistory[dragHistory.length - 2];
    if (h1 && h2) {
      const dt = (h1.t - h2.t) / 1000;
      const vx = dt > 0 ? pxToMeters(h1.x - h2.x) / dt : 0;
      const vy = dt > 0 ? pxToMeters(h1.y - h2.y) / dt : 0;
      draggingBody.setLinearVelocity(new planck.Vec2(vx, vy));
    }
  }
  dragHistory = [];

  if (elapsedTime < CONFIG.clickThresholdMs && movementSq < 100) {
    const userData = potentialClickBody.getUserData() as CustomBodyData;
    const clickedBadge = userData.badge;
    const relevantUsrBadges = props.usrBadges.filter(
      (ub) => ub.badgeID === clickedBadge.id && ub.to === props.usrID
    );
    onClickBadge(event as any, clickedBadge, relevantUsrBadges);
    emit("click-badge", clickedBadge, relevantUsrBadges);
  }
  potentialClickBody = null;
  draggingBody = null;
  dragOffset = null;
};

const handleMouseMove = (event: MouseEvent | Touch) => {
  if (!draggingBody || !dragOffset || !physicsWorld) return;
  const { x, y } = getMouseCoords(event);
  const newPos = new planck.Vec2(pxToMeters(x) - dragOffset.x, pxToMeters(y) - dragOffset.y);
  draggingBody.setLinearVelocity(new planck.Vec2(0, 0));
  draggingBody.setAngularVelocity(0);
  draggingBody.setPosition(newPos);
  dragHistory.push({ x, y, t: Date.now() });
  if (dragHistory.length > 6) dragHistory.shift();
};

const initializePhysics = (container: HTMLElement, canvas: HTMLCanvasElement, badges: Badge[]): PhysicsWorld => {
  const { clientWidth: width, clientHeight: height } = container;
  const world = new planck.World({ gravity: new planck.Vec2(0, pxToMeters(CONFIG.gravity)) });

  const pixelRatio = window.devicePixelRatio || 1;
  canvas.width = width * pixelRatio;
  canvas.height = height * pixelRatio;
  canvas.style.width = `${width}px`;
  canvas.style.height = `${height}px`;

  const widthM = pxToMeters(width);
  const heightM = pxToMeters(height);
  const wallThicknessM = pxToMeters(CONFIG.wallThickness);
  const wallOptions = { restitution: 1.0, friction: 0 };

  const ground = world.createBody({ position: new planck.Vec2(widthM / 2, heightM + wallThicknessM / 2) });
  ground.createFixture({ shape: new planck.Box(widthM / 2, wallThicknessM / 2), ...wallOptions });
  const left = world.createBody({ position: new planck.Vec2(-wallThicknessM / 2, heightM / 2) });
  left.createFixture({ shape: new planck.Box(wallThicknessM / 2, heightM / 2), ...wallOptions });
  const right = world.createBody({ position: new planck.Vec2(widthM + wallThicknessM / 2, heightM / 2) });
  right.createFixture({ shape: new planck.Box(wallThicknessM / 2, heightM / 2), ...wallOptions });
  const ceiling = world.createBody({ position: new planck.Vec2(widthM / 2, -wallThicknessM / 2) });
  ceiling.createFixture({ shape: new planck.Box(widthM / 2, wallThicknessM / 2), ...wallOptions });

  const walls = { ground, left, right, ceiling };

  // 4ランク分布: 型安全+存在保証
  const sizeMin = CONFIG.badgeRadiusFrom;
  const sizeMax = CONFIG.badgeRadiusTo;
  const rankNum = 4;
  const badgeCount = badges.length;

  const rankRanges = Array.from({ length: rankNum }, (_, i) => ({
    min: sizeMin + ((sizeMax - sizeMin) * i) / rankNum,
    max: sizeMin + ((sizeMax - sizeMin) * (i + 1)) / rankNum,
  }));

  const rankCounts = Array(rankNum).fill(Math.floor(badgeCount / rankNum));
  rankCounts[rankNum - 1] += badgeCount % rankNum;
  const badgeRanks: number[] = [];
  rankCounts.forEach((cnt, i) => {
    for (let j = 0; j < cnt; ++j) badgeRanks.push(i);
  });
  for (let i = badgeRanks.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    // 型安全シャッフル
    if (typeof badgeRanks[i] !== "undefined" && typeof badgeRanks[j] !== "undefined") {
      // tsエラー回避のため as で明示
      [badgeRanks[i], badgeRanks[j]] = [badgeRanks[j] as number, badgeRanks[i] as number];
    }
  }

  badges.forEach((badge, idx) => {
    const rank = badgeRanks[idx];
    const safeRank = typeof rank === "number" && rank >= 0 && rank < rankRanges.length ? rank : 0;
    const range = rankRanges[safeRank] ?? { min: CONFIG.badgeRadiusFrom, max: CONFIG.badgeRadiusTo };
    const badgeRadius = range.min + Math.random() * (range.max - range.min);

    const xPx = badgeRadius + Math.random() * (width - badgeRadius * 2);
    const yPx = badgeRadius + Math.random() * (height / 2);
    const direction: -1 | 1 = Math.random() < 0.5 ? -1 : 1;
    const velocity = 0.5 + Math.random() * 1.2;

    // tsエラー回避のために as で明示
    const color = (BALL_COLORS.length > 0 ? BALL_COLORS[idx % BALL_COLORS.length] : "#cccccc") as string;

    const body = world.createBody({
      type: 'dynamic',
      position: new planck.Vec2(pxToMeters(xPx), pxToMeters(yPx)),
      linearDamping: CONFIG.frictionAir,
      angularDamping: CONFIG.frictionAir
    });

    body.createFixture({
      shape: new planck.Circle(pxToMeters(badgeRadius)),
      density: 1.0,
      restitution: CONFIG.restitution,
      friction: CONFIG.friction
    });

    const customData: CustomBodyData = {
      id: badge.id,
      initialRadius: badgeRadius,
      content: badge.shortName ?? "",
      color,
      badge,
      direction,
      velocity,
    };
    body.setUserData(customData);
  });

  const startTime = performance.now();
  let animationFrameId: number | null = null;

  const step = () => {
    const elapsed = performance.now() - startTime;
    for (let body = world.getBodyList(); body; body = body.getNext()) {
      const userData = body.getUserData() as CustomBodyData | null;
      if (userData && body.isDynamic()) {
        // 1%の確率で方向転換＆速度再ランダム化
        if (Math.random() < 0.01) {
          userData.direction *= -1 as -1 | 1;
          userData.velocity = 1.0 + Math.random() * 0.35;
        }
        // 2秒経過してからだけ自動運動ON
        if (elapsed > 2000) {
          const currentVel = body.getLinearVelocity();
          body.setLinearVelocity(new planck.Vec2(userData.direction * userData.velocity, currentVel.y));
        }
      }
    }
    world.step(1 / 60, 8, 3);
    const newRenderedBadges: RenderedBadge[] = [];
    for (let body = world.getBodyList(); body; body = body.getNext()) {
      const userData = body.getUserData() as CustomBodyData | null;
      if (userData && body.isDynamic()) {
        const position = body.getPosition();
        const angle = body.getAngle();
        const displayRadius = userData.initialRadius;
        newRenderedBadges.push({
          id: userData.id,
          x: metersToPx(position.x),
          y: metersToPx(position.y),
          angle,
          radius: displayRadius,
          color: userData.color ?? "#cccccc",
          content: userData.content ?? "",
          badge: userData.badge
        });
      }
    }
    if (physicsWorld) physicsWorld.animationFrameId = requestAnimationFrame(step);
    renderedBadges.value = newRenderedBadges;
  };

  animationFrameId = requestAnimationFrame(step);
  return { world, walls, animationFrameId, startTime };
};

const cleanupPhysics = (physWorld: PhysicsWorld): void => {
  if (physWorld.animationFrameId !== null) {
    cancelAnimationFrame(physWorld.animationFrameId);
  }
  physWorld.world.destroyBody(physWorld.walls.ground);
  physWorld.world.destroyBody(physWorld.walls.left);
  physWorld.world.destroyBody(physWorld.walls.right);
  physWorld.world.destroyBody(physWorld.walls.ceiling);
  let body = physWorld.world.getBodyList();
  while (body) {
    const next = body.getNext();
    physWorld.world.destroyBody(body);
    body = next;
  }
};

import { nextTick } from 'vue';

onMounted(async () => {
  if (!containerRef.value || !canvasRef.value) return;
  await nextTick();
  await sleep(50); // レイアウトの安定を待つ
  physicsWorld = initializePhysics(containerRef.value, canvasRef.value, props.badges);
  containerRef.value.addEventListener("mousedown", (e) => handleMouseDown(e));
  containerRef.value.addEventListener("mouseup", (e) => handleMouseUp(e));
  containerRef.value.addEventListener("mousemove", (e) => handleMouseMove(e));
  containerRef.value.addEventListener("touchstart", (e) => {
    const touch = e.touches[0];
    if (touch) handleMouseDown(touch);
  });
  containerRef.value.addEventListener("touchend", (e) => {
    const touch = e.changedTouches[0];
    if (touch) handleMouseUp(touch);
  });
  containerRef.value.addEventListener("touchmove", (e) => {
    const touch = e.touches[0];
    if (touch) handleMouseMove(touch);
  });
  window.addEventListener("resize", handleResize);
});

onUnmounted(() => {
  window.removeEventListener("resize", handleResize);
  if (containerRef.value) {
    containerRef.value.removeEventListener("mousedown", handleMouseDown as EventListener);
    containerRef.value.removeEventListener("mouseup", handleMouseUp as EventListener);
    containerRef.value.removeEventListener("mousemove", handleMouseMove as EventListener);
    // touchリスナーは匿名なので消さなくてもOK
  }
  if (physicsWorld) {
    cleanupPhysics(physicsWorld);
    physicsWorld = null;
  }
});

const onClickBadge = async (e: MouseEvent | Touch, badge: Badge, usrBadges: UsrBadge[]) => {
  if (!badgeWindowRef.value) return;
  currentBadge.value = badge;
  currentUsrBadges.value = usrBadges;
  isBadgeWindowOpen.value = true;
  const coords = getMouseCoords(e);
  badgeWindowRef.value.style.left = `${coords.x}px`;
  badgeWindowRef.value.style.top = `${coords.y}px`;
  await sleep(30);
  const centerClass = "__harunohi-badge-window-center";
  const openClass = "__harunohi-badge-window-open";
  const compClass = "__harunohi-badge-window-comp";
  badgeWindowRef.value.classList.add(centerClass);
  await sleep(400);
  badgeWindowRef.value.classList.add(openClass);
  await sleep(300);
  badgeWindowRef.value.classList.add(compClass);
  await sleep(300);
  isBadgeContentDialogOpen.value = true;
  await sleep(300);
  if (physicsWorld) { cleanupPhysics(physicsWorld) }
};

const onCloseBadge = async () => {
  if (!badgeWindowRef.value) return;
  if (!containerRef.value || !canvasRef.value) return;
  const centerClass = "__harunohi-badge-window-center";
  const openClass = "__harunohi-badge-window-open";
  const compClass = "__harunohi-badge-window-comp";
  await sleep(300);
  badgeWindowRef.value.classList.remove(compClass);
  await sleep(300);
  physicsWorld = initializePhysics(containerRef.value, canvasRef.value, props.badges);
  badgeWindowRef.value.classList.remove(openClass);
  await sleep(400);
  badgeWindowRef.value.classList.remove(centerClass);
  await sleep(300);
  isBadgeWindowOpen.value = false;
};

const tab = computed(() => mainStore.tab)

if (tab.value !== TAB.BADGE && physicsWorld) { cleanupPhysics(physicsWorld) }

watch(isBadgeContentDialogOpen, (n) => {
  if (n) return;
  onCloseBadge();
});
watch(tab, (n) => {
  if (n !== TAB.BADGE && physicsWorld) { cleanupPhysics(physicsWorld) }
})
</script>

<style scoped lang="scss">
.animation-container {
  width: 100%;
  height: 100%;
  &.is-tauri {
    height: calc(100dvh - var(--tauri-offset-total)) !important;
    min-height: calc(100dvh - var(--tauri-offset-total)) !important;
  }
  overflow: hidden;
  position: relative;
  touch-action: none;
}
canvas {
  display: block;
  width: 100%;
  height: 100%;
  position: absolute;
  top: 0;
  left: 0;
  opacity: 0;
  pointer-events: none;
}
.visual-badge {
  position: absolute;
  top: 0;
  left: 0;
  border-radius: 50%;
  border: 2px solid #fff;
  display: flex;
  justify-content: center;
  align-items: center;
  user-select: none;
  pointer-events: none;
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
  font-weight: bold;
  font-size: 12px;
  color: #fff;
  text-shadow: 1px 1px 5px rgba(0, 0, 0, 0.4);
  will-change: transform;
  z-index: 1;
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2), inset 0 -10px 15px -5px rgba(0, 0, 0, 0.07);
}
.badge-content {
  pointer-events: none;
  white-space: nowrap;
}
$harunohi-window-badge-default-width: 30px;
.__harunohi-badge-window {
  position: fixed;
  top: 0;
  left: 0;
  width: $harunohi-window-badge-default-width;
  height: $harunohi-window-badge-default-width;
  border-radius: calc($harunohi-window-badge-default-width / 2);
  background-color: $secondary;
  overflow: hidden;
  z-index: 2;
  display: none;
  &-visible {
    transition: all 0.3s ease;
    display: block !important;
  }
  &-center {
    top: 20px !important;
    left: calc(100dvw / 2 - $harunohi-window-badge-default-width / 2) !important;
  }
  &-open {
    top: 0 !important;
    left: 0 !important;
    width: 100dvw !important;
    height: 100dvh !important;
    border-radius: 0 !important;
    background-color: $secondary-light !important;
  }
  &-comp {
    background-color: $primary-light !important;
  }
  &-wrap {
    position: relative;
    width: 100%;
    height: 100%;
  }
}
</style>
