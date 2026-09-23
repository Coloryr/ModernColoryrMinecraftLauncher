// 生成新闻占位配图（SVG data URI，无需网络）
// 接入后端后替换为真实图片 URL 即可（NewsItem.image）

function svgBanner(c1: string, c2: string, seed: number): string {
  // 由 seed 决定装饰方块的位置，保证同一条新闻图片稳定
  const rnd = (i: number) => {
    const x = Math.sin(seed * 127.1 + i * 311.7) * 43758.5453;
    return x - Math.floor(x);
  };

  let blocks = "";
  for (let i = 0; i < 6; i++) {
    const x = Math.floor(30 + rnd(i) * 560);
    const y = Math.floor(30 + rnd(i + 6) * 120);
    const s = 30 + Math.floor(rnd(i + 12) * 60);
    const o = 0.1 + rnd(i + 18) * 0.16;
    const r = 8 + Math.floor(rnd(i + 24) * 10);
    blocks += `<rect x="${x}" y="${y}" width="${s}" height="${s}" rx="${r}" fill="rgba(255,255,255,${o.toFixed(2)})"/>`;
  }

  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="200">` +
    `<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">` +
    `<stop offset="0" stop-color="${c1}"/><stop offset="1" stop-color="${c2}"/>` +
    `</linearGradient></defs>` +
    `<rect width="640" height="200" fill="url(#g)"/>` +
    blocks +
    `</svg>`;

  return "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
}

/** 生成一组新闻配图（返回图片字符串） */
export function newsImage(seed: number, c1: string, c2: string): string {
  return svgBanner(c1, c2, seed);
}
