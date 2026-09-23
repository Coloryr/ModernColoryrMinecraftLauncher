// 账户皮肤 / 头像 / 披风占位图（SVG data URI，无需网络）
// 接入真实皮肤服务后替换为实际图片 URL

function svg(w: number, h: number, body: string): string {
  const data =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">${body}</svg>`;
  return "data:image/svg+xml;charset=utf-8," + encodeURIComponent(data);
}

function seedColor(seed: number, salt: number): string {
  const h = ((seed * 2654435761 + salt * 40503) >>> 0).toString(16).padStart(6, "0");
  return h.slice(0, 6);
}

/** 头像（64x64） */
export function avatarImage(seed: number, skin: string): string {
  const hair = seedColor(seed, 1);
  const bg = seedColor(seed, 2);
  return svg(
    64,
    64,
    `<rect width="64" height="64" fill="#${bg}"/>` +
      `<rect x="14" y="12" width="36" height="38" rx="7" fill="${skin}"/>` +
      `<rect x="14" y="12" width="36" height="12" rx="4" fill="#${hair}"/>` +
      `<rect x="22" y="28" width="8" height="10" rx="2" fill="#fff"/><rect x="34" y="28" width="8" height="10" rx="2" fill="#fff"/>` +
      `<rect x="24" y="30" width="4" height="6" rx="1" fill="#1f2937"/><rect x="36" y="30" width="4" height="6" rx="1" fill="#1f2937"/>` +
      `<rect x="26" y="40" width="12" height="4" rx="2" fill="#1f2937"/>`,
  );
}

/** 皮肤（64x128，正面全身） */
export function skinImage(seed: number, skin: string): string {
  const hair = seedColor(seed, 3);
  const shirt = seedColor(seed, 4);
  const bg = seedColor(seed, 5);
  return svg(
    64,
    128,
    `<rect width="64" height="128" fill="#${bg}"/>` +
      `<rect x="20" y="6" width="24" height="24" rx="3" fill="${skin}"/>` +
      `<rect x="20" y="6" width="24" height="8" rx="3" fill="#${hair}"/>` +
      `<rect x="24" y="18" width="5" height="5" rx="1" fill="#fff"/><rect x="35" y="18" width="5" height="5" rx="1" fill="#fff"/>` +
      `<rect x="18" y="32" width="28" height="38" rx="4" fill="#${shirt}"/>` +
      `<rect x="6" y="32" width="10" height="36" rx="4" fill="#${shirt}"/>` +
      `<rect x="48" y="32" width="10" height="36" rx="4" fill="#${shirt}"/>` +
      `<rect x="20" y="72" width="11" height="30" rx="4" fill="#334155"/>` +
      `<rect x="33" y="72" width="11" height="30" rx="4" fill="#334155"/>`,
  );
}

/** 披风（64x32） */
export function capeImage(seed: number, skin: string): string {
  const c1 = seedColor(seed, 6);
  const c2 = seedColor(seed, 7);
  return svg(
    64,
    32,
    `<rect width="64" height="32" fill="#${c1}"/>` +
      `<rect x="10" y="6" width="12" height="22" rx="3" fill="${skin}"/>` +
      `<rect x="26" y="6" width="12" height="22" rx="3" fill="#${c2}"/>` +
      `<rect x="42" y="6" width="12" height="22" rx="3" fill="#${c2}"/>`,
  );
}
