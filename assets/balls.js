document.addEventListener("DOMContentLoaded", function () {
  var Engine = Matter.Engine,
    Render = Matter.Render,
    Runner = Matter.Runner,
    Bodies = Matter.Bodies,
    Body = Matter.Body,
    Composite = Matter.Composite;

  const canvas = document.getElementById('ballsCanvas');
  let w = canvas.offsetWidth;
  let h = canvas.offsetHeight;

  const engine = Engine.create();
  const render = Render.create({
    engine: engine,
    canvas: canvas,
    options: {
      width: w,
      height: h,
      background: 'transparent',
      wireframes: false,
      pixelRatio: window.devicePixelRatio,
      //showPerformance: true,
    }
  });

  let removedStripe = 100;
  let ratio = h / w;

  Render.lookAt(render, {
    min: { x: 0, y: 0 },
    max: { x: w - removedStripe, y: h - (removedStripe * ratio) }
  });

  engine.world.gravity.x = -0.35;
  engine.world.gravity.y = -0.5;

  const boundariesOptions = {
    isStatic: true,
    render: { visible: false }
  };

  const boundaries = [
    // Top Boundary
    Bodies.rectangle(w / 2, -3, w, 10, boundariesOptions),
    // Bottom Boundary
    Bodies.rectangle(w / 2, h + 3, w, 10, boundariesOptions),
    // Left Boundary
    Bodies.rectangle(-3, h / 2, 10, h, boundariesOptions),
    // Right Boundary
    Bodies.rectangle(w + 3, h / 2, 10, h, boundariesOptions),
  ];

  Composite.add(engine.world, boundaries);

  function spawnComposite(isCube) {
    const size = 6;

    const colors = ['#ff5caa','#4f55e3','#ff4e62','#ad6de6'];
    const yellow = '#ffdf00';

    const spawnX = w - Math.random() * (removedStripe / 2);
    const spawnY = h - Math.random() * (h / 2) - removedStripe * ratio;

    const color = isCube ? yellow : colors[Math.floor(Math.random() * colors.length)];
    const options = {
      label: 'ball',
      restitution: 0.8,
      render: { fillStyle: color }
    };

    let object;
    if (isCube) {
      object = Bodies.rectangle(spawnX, spawnY, size * 2, size * 2, options);
    } else {
      object = Bodies.circle(spawnX, spawnY, size, options);
    }
    Composite.add(engine.world, object);
  }

  const count = 70;
  const spawnDurationMs = 3000;
  const cubeGenerated = false;
  for (let i = 0; i < count; i++) {
    setTimeout(() => {
      let isCube = (i == Math.floor(count / 2));
      spawnComposite(isCube);
    }, i * (spawnDurationMs / count));
  }

  // run the renderer
  Render.run(render);

  // create runner
  var runner = Runner.create();

  // run the engine
  Runner.run(runner, engine);
});
