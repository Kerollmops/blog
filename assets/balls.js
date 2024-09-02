const loadImage = (url, onSuccess, onError) => {
  const img = new Image();
  img.onload = () => {
    onSuccess(img.src);
  };
  img.onerror = onError();
  img.src = url;
};

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

  function spawnComposite(image_url) {
    const size = 8;
    const scale = 0.18;
    const spawnX = w - Math.random() * (removedStripe / 2);
    const spawnY = h - Math.random() * (h / 2) - removedStripe * ratio;

    const options = {
      label: 'key',
      restitution: 0.8,
      render: { sprite: { texture: image_url, xScale: scale, yScale: scale } }
    };

    const object = Bodies.rectangle(spawnX, spawnY, size * 2, size * 2, options);
    Composite.add(engine.world, object);
  }

    const count = Math.random() >= 0.5 ? 3 : 4;
    const spawnDurationMs = 1000;
    const keys = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

    for (let i = 0; i < count; i++) {
      const key = keys[Math.floor(Math.random() * keys.length)];
      const url = `/assets/keys/${key}.png`
      loadImage(url, (image_url) => {
        setTimeout(() => spawnComposite(image_url), i * (spawnDurationMs / count));
      }, (e) => console.log(e));
    }

  // run the renderer
  Render.run(render);

  // create runner
  var runner = Runner.create();

  // run the engine
  Runner.run(runner, engine);
});
