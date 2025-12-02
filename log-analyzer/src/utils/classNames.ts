// 样式类名合并工具
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/**
 * 合并Tailwind CSS类名,处理冲突
 * @param inputs - 类名输入(支持条件类名)
 * @returns 合并后的类名字符串
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
