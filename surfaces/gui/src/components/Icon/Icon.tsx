import type { IconName } from "./IconName";
import { svgProps } from "./svgProps";
import { glyphs1 } from "./glyphs1";
import { glyphs2 } from "./glyphs2";
import { glyphs3 } from "./glyphs3";

export function Icon({
  name,
  size = 16,
  className,
}: {
  name: IconName;
  size?: number;
  className?: string;
}) {
  const s = svgProps(size, className);

  return glyphs1(name, s) ?? glyphs2(name, s) ?? glyphs3(name, s);
}
