const HEX_PATTERN = /#([0-9a-fA-F]){6}/;

export function hex2rgb(hex: string) {
  if (hex.length === 7 && HEX_PATTERN.test(hex)) {
    const h = hex.substring(1);
    const r = parseInt(h.slice(0, 2), 16);
    const g = parseInt(h.slice(2, 4), 16);
    const b = parseInt(h.slice(4, 6), 16);
    return `${r} ${g} ${b}`;
  }

  return "0 0 0";
}

export function rgb2hex(rgb: string) {
  const splitted = rgb.split(" ");
  if (splitted.length === 3) {
    let result = "#";
    for (const c of splitted) {
      const ch = parseInt(c, 10).toString(16);

      result += ch.length === 1 ? `0${ch}` : ch;
    }
    return result;
  }

  return "#ffffff";
}
