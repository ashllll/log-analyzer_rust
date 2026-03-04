#version 320 es

/// 热力图片段着色器
///
/// PRD V6.0 4.2 GPU 着色器缩略图
/// 将 Rust 端 density_map (Uint8List) 直接传递给 GPU
/// 实现纳秒级计算滚动条像素颜色热力图
///
/// 精度说明:
/// - density 值范围: 0-255 (Uint8)
/// - 热力图颜色映射: 蓝(冷) -> 绿 -> 黄 -> 红(热)

precision highp float;

// 统一变量 (从 Flutter 传入)
uniform sampler2D u_density_texture;  // 密度纹理 (从 density_map 生成)
uniform vec2 u_resolution;            // 画布分辨率
uniform float u_max_density;          // 最大密度值 (用于归一化)

// 输出变量
out vec4 fragColor;

/// 热力图颜色映射函数
///
/// 将归一化的密度值 (0.0-1.0) 映射到热力图颜色
/// 使用平滑插值实现渐变效果
vec3 heatmapColor(float t) {
    // 五点渐变: 蓝 -> 青 -> 绿 -> 黄 -> 红
    vec3 blue = vec3(0.0, 0.0, 1.0);
    vec3 cyan = vec3(0.0, 1.0, 1.0);
    vec3 green = vec3(0.0, 1.0, 0.0);
    vec3 yellow = vec3(1.0, 1.0, 0.0);
    vec3 red = vec3(1.0, 0.0, 0.0);

    vec3 color;

    if (t < 0.25) {
        // 蓝 -> 青
        color = mix(blue, cyan, t * 4.0);
    } else if (t < 0.5) {
        // 青 -> 绿
        color = mix(cyan, green, (t - 0.25) * 4.0);
    } else if (t < 0.75) {
        // 绿 -> 黄
        color = mix(green, yellow, (t - 0.5) * 4.0);
    } else {
        // 黄 -> 红
        color = mix(yellow, red, (t - 0.75) * 4.0);
    }

    return color;
}

void main() {
    // 使用 gl_FragCoord 计算纹理坐标 (Flutter SkSL 兼容)
    // gl_FragCoord.xy 是像素坐标，需要归一化到 [0,1]
    vec2 fragCoord = gl_FragCoord.xy;
    vec2 texCoord = fragCoord / u_resolution;

    // 从纹理采样密度值
    // 纹理坐标: texCoord.y 从底部(0)到顶部(1)
    vec4 densitySample = texture(u_density_texture, vec2(0.5, texCoord.y));

    // 提取密度值 (使用红色通道)
    float density = densitySample.r;

    // 归一化密度值 (0.0 - 1.0)
    float normalizedDensity = density / max(u_max_density, 1.0);

    // 应用伽马校正增强对比度
    normalizedDensity = pow(normalizedDensity, 0.7);

    // 获取热力图颜色
    vec3 color = heatmapColor(normalizedDensity);

    // 输出最终颜色 (带透明度)
    // 低密度区域更透明，高密度区域更不透明
    float alpha = 0.3 + normalizedDensity * 0.7;
    fragColor = vec4(color, alpha);
}
