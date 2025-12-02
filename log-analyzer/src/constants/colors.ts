// 颜色系统常量定义
import { ColorKey } from '../types/common';

// 颜色样式映射
export const COLOR_STYLES: Record<ColorKey, any> = {
  blue: {
    dot: "bg-blue-500",
    badge: "bg-blue-500/15 text-blue-400 border-blue-500/20",
    border: "border-blue-500",
    text: "text-blue-400",
    activeBtn: "bg-blue-500 text-white border-blue-400 shadow-[0_0_10px_rgba(59,130,246,0.4)]",
    hoverBorder: "hover:border-blue-500/50",
    highlight: "bg-blue-500/20 text-blue-300 border-blue-500/30"
  },
  green: {
    dot: "bg-emerald-500",
    badge: "bg-emerald-500/15 text-emerald-400 border-emerald-500/20",
    border: "border-emerald-500",
    text: "text-emerald-400",
    activeBtn: "bg-emerald-500 text-white border-emerald-400 shadow-[0_0_10px_rgba(16,185,129,0.4)]",
    hoverBorder: "hover:border-emerald-500/50",
    highlight: "bg-emerald-500/20 text-emerald-300 border-emerald-500/30"
  },
  red: {
    dot: "bg-red-500",
    badge: "bg-red-500/15 text-red-400 border-red-500/20",
    border: "border-red-500",
    text: "text-red-400",
    activeBtn: "bg-red-500 text-white border-red-400 shadow-[0_0_10px_rgba(239,68,68,0.4)]",
    hoverBorder: "hover:border-red-500/50",
    highlight: "bg-red-500/20 text-red-300 border-red-500/30"
  },
  orange: {
    dot: "bg-amber-500",
    badge: "bg-amber-500/15 text-amber-400 border-amber-500/20",
    border: "border-amber-500",
    text: "text-amber-400",
    activeBtn: "bg-amber-500 text-white border-amber-400 shadow-[0_0_10px_rgba(245,158,11,0.4)]",
    hoverBorder: "hover:border-amber-500/50",
    highlight: "bg-amber-500/20 text-amber-300 border-amber-500/30"
  },
  purple: {
    dot: "bg-purple-500",
    badge: "bg-purple-500/15 text-purple-400 border-purple-500/20",
    border: "border-purple-500",
    text: "text-purple-400",
    activeBtn: "bg-purple-500 text-white border-purple-400 shadow-[0_0_10px_rgba(168,85,247,0.4)]",
    hoverBorder: "hover:border-purple-500/50",
    highlight: "bg-purple-500/20 text-purple-300 border-purple-500/30"
  }
};
