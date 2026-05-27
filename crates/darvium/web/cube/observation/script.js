const app = new PIXI.Application();

(async () => {
  await app.init({
    resizeTo: window,
    background: "#efefef",
    antialias: true,
  });

  document.getElementById("app").appendChild(app.canvas);

  const layer = new PIXI.Container();
  app.stage.addChild(layer);

  const circles = [];
  const maxCircles = 500;
  const spawnPerFrame = 8;

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

  function getColor(radius) {
    const red = 0xff2b2b;
    const blue = 0x2b6cff;
    const black = 0x000000;

    if (radius < 20) {
      const t = radius / 20;
      return lerpColor(red, blue, t);
    }

    const t = Math.min((radius - 20) / 40, 1);
    return lerpColor(blue, black, t);
  }

  function spawnCircle() {
    const g = new PIXI.Graphics();
    layer.addChild(g);

    circles.push({
      x: Math.random() * app.screen.width,
      y: Math.random() * app.screen.height,
      angle: Math.random() * Math.PI * 2,
      speed: 0.4 + Math.random() * 1.0,
      turnRate: 0.03 + Math.random() * 0.05,
      radius: 2 + Math.random() * 3,
      growth: 0.08 + Math.random() * 0.12,
      graphics: g,
    });
  }

  app.ticker.add((ticker) => {
    const dt = ticker.elapsedMS / 16.6667;

    for (let i = 0; i < spawnPerFrame; i++) {
      if (circles.length < maxCircles) spawnCircle();
    }

    for (let i = circles.length - 1; i >= 0; i--) {
      const c = circles[i];

      c.angle += (Math.random() - 0.5) * c.turnRate * dt;

      c.x += Math.cos(c.angle) * c.speed * dt;
      c.y += Math.sin(c.angle) * c.speed * dt;
      c.radius += c.growth * dt;

      if (c.x < 0) {
        c.x = 0;
        c.angle = Math.PI - c.angle;
      } else if (c.x > app.screen.width) {
        c.x = app.screen.width;
        c.angle = Math.PI - c.angle;
      }

      if (c.y < 0) {
        c.y = 0;
        c.angle = -c.angle;
      } else if (c.y > app.screen.height) {
        c.y = app.screen.height;
        c.angle = -c.angle;
      }

      const color = getColor(c.radius);

      c.graphics.clear();
      c.graphics.circle(0, 0, c.radius);
      c.graphics.fill(color);
      c.graphics.x = c.x;
      c.graphics.y = c.y;

      if (c.radius > 60) {
        layer.removeChild(c.graphics);
        c.graphics.destroy();
        circles.splice(i, 1);
      }
    }
  });
})();
