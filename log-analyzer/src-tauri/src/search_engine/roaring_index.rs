//! Roaring Bitmap 搜索结果压缩模块
//!
//! 该模块实现了高效的搜索结果位图索引，用于：
//! - 压缩存储大量搜索命中结果（行号）
//! - O(1) 复杂度的 select(k) 操作，获取第 k 个命中行
//! - 生成视口密度图用于 UI 渲染
//!
//! ## 性能特性
//! - 千万级结果压缩至 < 5MB
//! - select(k) 操作 O(1) 时间复杂度（使用预计算索引数组）
//! - 迭代操作 O(n) 时间复杂度
//!
//! ## PRD 3.2 要求
//! - 命中数据全部压缩在 Rust 端的 RoaringBitmap 内
//! - Dart 端接收 total_hits，根据当前视口发起 O(1) 复杂度的 select(k) 请求

use roaring::RoaringBitmap;
use serde::{Deserialize, Serialize};

/// 搜索索引 - 使用 Roaring Bitmap 存储命中行号
///
/// Roaring Bitmap 是一种高效的压缩位图实现，特别适合存储稀疏的整数集合。
/// 它结合了多种压缩技术，在内存占用和查询性能之间取得良好平衡。
///
/// ## O(1) select(k) 实现
/// 使用预计算的索引数组（`sorted_hits`）实现 O(1) 复杂度的 select 操作。
/// 当调用 `freeze()` 后，会将 RoaringBitmap 中的值展开为数组，
/// 后续的 select(k) 操作直接通过数组索引完成，复杂度为 O(1)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndex {
    /// 命中的行号集合（压缩存储）
    hits: RoaringBitmap,
    /// 预计算的有序命中行号数组（用于 O(1) select）
    /// 调用 freeze() 后才会填充此字段
    #[serde(default)]
    sorted_hits: Option<Vec<u64>>,
    /// 总行数（用于计算密度）
    total_lines: u64,
    /// 是否已冻结（已预计算索引数组）
    #[serde(default)]
    frozen: bool,
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchIndex {
    /// 创建新的空搜索索引
    ///
    /// # 示例
    /// ```
    /// use log_analyzer::search_engine::SearchIndex;
    ///
    /// let index = SearchIndex::new();
    /// assert_eq!(index.len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            hits: RoaringBitmap::new(),
            sorted_hits: None,
            total_lines: 0,
            frozen: false,
        }
    }

    /// 创建指定总行数的空搜索索引
    ///
    /// # 参数
    /// - `total_lines`: 文件总行数，用于密度计算
    ///
    /// # 示例
    /// ```
    /// use log_analyzer::search_engine::SearchIndex;
    ///
    /// let index = SearchIndex::with_total_lines(10000);
    /// assert_eq!(index.total_lines(), 10000);
    /// ```
    pub fn with_total_lines(total_lines: u64) -> Self {
        Self {
            hits: RoaringBitmap::new(),
            sorted_hits: None,
            total_lines,
            frozen: false,
        }
    }

    /// 冻结索引，预计算有序数组以实现 O(1) select
    ///
    /// 调用此方法后，会将 RoaringBitmap 中的值展开为有序数组。
    /// 后续的 select(k) 操作将变为 O(1) 复杂度。
    ///
    /// # 复杂度
    /// - 时间: O(n) 其中 n 是命中数量
    /// - 空间: O(n) 额外的数组存储
    ///
    /// # 示例
    /// ```
    /// use log_analyzer::search_engine::SearchIndex;
    ///
    /// let mut index = SearchIndex::new();
    /// index.add_hits([5, 10, 15, 20]);
    /// index.freeze();  // 预计算后 select(k) 变为 O(1)
    ///
    /// assert_eq!(index.select(0), Some(5));
    /// ```
    pub fn freeze(&mut self) {
        if self.frozen {
            return;
        }

        // 将 RoaringBitmap 展开为有序数组
        let sorted: Vec<u64> = self.hits.iter().map(|n| n as u64).collect();
        self.sorted_hits = Some(sorted);
        self.frozen = true;
    }

    /// 解冻索引，释放预计算的数组
    ///
    /// 在需要继续添加命中时调用。
    pub fn unfreeze(&mut self) {
        self.sorted_hits = None;
        self.frozen = false;
    }

    /// 检查索引是否已冻结
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// 添加一个命中行号
    ///
    /// # 参数
    /// - `line_number`: 命中的行号（从 0 开始）
    ///
    /// # 复杂度
    /// - 时间: O(1) 均摊
    /// - 空间: 取决于位图压缩效率
    ///
    /// # 注意
    /// 如果索引已冻结，会自动解冻。
    ///
    /// # Panics
    /// 如果 line_number 超过 u32::MAX，会 panic
    pub fn add_hit(&mut self, line_number: u64) {
        if self.frozen {
            self.unfreeze();
        }
        self.hits.insert(
            line_number
                .try_into()
                .expect("line_number exceeds u32::MAX"),
        );
    }

    /// 批量添加命中行号
    ///
    /// # 参数
    /// - `line_numbers`: 命中的行号迭代器
    ///
    /// # 复杂度
    /// - 时间: O(n) 其中 n 是行号数量
    ///
    /// # 注意
    /// 如果索引已冻结，会自动解冻。
    ///
    /// # Panics
    /// 如果任何 line_number 超过 u32::MAX，会 panic
    pub fn add_hits<I: IntoIterator<Item = u64>>(&mut self, line_numbers: I) {
        if self.frozen {
            self.unfreeze();
        }
        for line in line_numbers {
            self.hits
                .insert(line.try_into().expect("line_number exceeds u32::MAX"));
        }
    }

    /// 设置总行数
    ///
    /// # 参数
    /// - `total_lines`: 文件总行数
    pub fn set_total_lines(&mut self, total_lines: u64) {
        self.total_lines = total_lines;
    }

    /// 检查指定行号是否命中
    ///
    /// # 参数
    /// - `line_number`: 要检查的行号
    ///
    /// # 返回
    /// - `true`: 该行是命中行
    /// - `false`: 该行不是命中行
    ///
    /// # 复杂度
    /// - 时间: O(1)
    pub fn contains(&self, line_number: u64) -> bool {
        line_number
            .try_into()
            .map(|n| self.hits.contains(n))
            .unwrap_or(false)
    }

    /// 获取第 k 个命中行号（O(1) select 操作）
    ///
    /// 这是 PRD 3.2 要求的核心操作，用于 Dart 端根据视口请求特定命中行。
    ///
    /// # 参数
    /// - `k`: 索引（从 0 开始），0 表示第 1 个命中行
    ///
    /// # 返回
    /// - `Some(line_number)`: 第 k 个命中行的行号
    /// - `None`: k 超出范围
    ///
    /// # 复杂度
    /// - 已冻结: O(1) - 直接数组索引
    /// - 未冻结: O(k) - 迭代到第 k 个元素
    ///
    /// # 建议
    /// 对于大量查询，建议先调用 `freeze()` 以获得 O(1) 复杂度。
    ///
    /// # 示例
    /// ```
    /// use log_analyzer::search_engine::SearchIndex;
    ///
    /// let mut index = SearchIndex::new();
    /// index.add_hits([5, 10, 15, 20]);
    /// index.freeze();  // 预计算后 select(k) 变为 O(1)
    ///
    /// assert_eq!(index.select(0), Some(5));   // 第 1 个命中
    /// assert_eq!(index.select(1), Some(10));  // 第 2 个命中
    /// assert_eq!(index.select(3), Some(20));  // 第 4 个命中
    /// assert_eq!(index.select(4), None);      // 超出范围
    /// ```
    pub fn select(&self, k: u64) -> Option<u64> {
        let len = self.hits.len();
        if k >= len {
            return None;
        }

        // 如果已冻结，使用 O(1) 数组索引
        if let Some(ref sorted) = self.sorted_hits {
            // 安全：我们已检查 k < len，且 sorted.len() == len
            return sorted.get(k as usize).copied();
        }

        // 未冻结时，使用 O(k) 迭代（兼容旧行为）
        self.hits.iter().nth(k as usize).map(|n| n as u64)
    }

    /// 批量获取命中行号（用于视口拉取）
    ///
    /// # 参数
    /// - `start_k`: 起始索引（包含）
    /// - `count`: 要获取的数量
    ///
    /// # 返回
    /// - 从第 start_k 个命中行开始的 count 个行号
    ///
    /// # 复杂度
    /// - 已冻结: O(count) - 直接数组切片
    /// - 未冻结: O(start_k + count)
    ///
    /// # 示例
    /// ```
    /// use log_analyzer::search_engine::SearchIndex;
    ///
    /// let mut index = SearchIndex::new();
    /// index.add_hits([5, 10, 15, 20, 25, 30]);
    /// index.freeze();  // 预计算后 select_range 变为 O(count)
    ///
    /// let batch = index.select_range(1, 3);
    /// assert_eq!(batch, vec![10, 15, 20]);
    /// ```
    pub fn select_range(&self, start_k: u64, count: u64) -> Vec<u64> {
        let total = self.hits.len();
        if start_k >= total {
            return Vec::new();
        }

        let end_k = std::cmp::min(start_k + count, total);
        let actual_count = (end_k - start_k) as usize;

        // 如果已冻结，使用 O(count) 数组切片
        if let Some(ref sorted) = self.sorted_hits {
            let start = start_k as usize;
            let end = start + actual_count;
            return sorted[start..end].to_vec();
        }

        // 未冻结时，使用 O(start_k + count) 迭代
        let mut result = Vec::with_capacity(actual_count);
        let mut iter = self.hits.iter();

        // 跳过前 start_k 个元素
        for _ in 0..start_k {
            iter.next();
        }

        // 收集接下来的 count 个元素
        for _ in 0..actual_count {
            if let Some(line) = iter.next() {
                result.push(line as u64);
            }
        }

        result
    }

    /// 获取命中行总数
    ///
    /// # 返回
    /// - 命中的行数（即 RoaringBitmap 的大小）
    ///
    /// # 复杂度
    /// - 时间: O(1)
    pub fn len(&self) -> u64 {
        self.hits.len()
    }

    /// 检查索引是否为空
    ///
    /// # 复杂度
    /// - 时间: O(1)
    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }

    /// 获取总行数
    pub fn total_lines(&self) -> u64 {
        self.total_lines
    }

    /// 获取命中密度（命中行数 / 总行数）
    ///
    /// # 返回
    /// - 0.0 到 1.0 之间的密度值
    pub fn density(&self) -> f64 {
        if self.total_lines == 0 {
            return 0.0;
        }
        self.hits.len() as f64 / self.total_lines as f64
    }

    /// 生成视口密度图（用于 UI 渲染）
    ///
    /// 将整个文件映射到指定高度的视口上，每个像素表示该区域是否有命中。
    ///
    /// # 参数
    /// - `viewport_height`: 视口高度（像素数）
    ///
    /// # 返回
    /// - `Vec<u8>`: 密度图，0 表示无命中，1 表示有命中
    ///
    /// # 算法
    /// 将文件按行数均匀划分为 viewport_height 个区间，
    /// 每个区间检查是否有命中行。
    ///
    /// # 示例
    /// ```
    /// use log_analyzer::search_engine::SearchIndex;
    ///
    /// let mut index = SearchIndex::with_total_lines(1000);
    /// index.add_hits([100, 300, 500, 700, 900]);
    ///
    /// // 生成 10 像素高的密度图
    /// let density = index.to_density_map(10);
    /// assert_eq!(density.len(), 10);
    /// // 每 100 行对应一个像素，应该有 5 个像素为 1
    /// assert_eq!(density.iter().filter(|&&v| v == 1).count(), 5);
    /// ```
    pub fn to_density_map(&self, viewport_height: u32) -> Vec<u8> {
        if self.total_lines == 0 || viewport_height == 0 {
            return vec![0; viewport_height as usize];
        }

        let viewport_height = viewport_height as u64;
        let mut density = vec![0u8; viewport_height as usize];

        // 计算每个像素对应的行数区间
        let lines_per_pixel = std::cmp::max(1, self.total_lines / viewport_height);

        // 遍历所有命中行，映射到对应的像素
        for hit_line in &self.hits {
            let pixel_index = ((hit_line as u64) / lines_per_pixel) as usize;
            if pixel_index < density.len() {
                density[pixel_index] = 1;
            }
        }

        density
    }

    /// 生成带强度的密度图（用于热力图渲染）
    ///
    /// # 参数
    /// - `viewport_height`: 视口高度（像素数）
    ///
    /// # 返回
    /// - `Vec<u8>`: 密度图，值表示该区域的命中数量（0-255）
    ///
    /// # 示例
    /// ```
    /// use log_analyzer::search_engine::SearchIndex;
    ///
    /// let mut index = SearchIndex::with_total_lines(1000);
    /// index.add_hits([100, 101, 102, 300, 500]);
    ///
    /// let heatmap = index.to_heatmap(10);
    /// assert_eq!(heatmap.len(), 10);
    /// // 第一个区间（行 0-99）应该有 3 个命中
    /// // 注意：具体值取决于 lines_per_pixel 的计算
    /// ```
    pub fn to_heatmap(&self, viewport_height: u32) -> Vec<u8> {
        if self.total_lines == 0 || viewport_height == 0 {
            return vec![0; viewport_height as usize];
        }

        let viewport_height = viewport_height as u64;
        let mut counts = vec![0u32; viewport_height as usize];

        // 计算每个像素对应的行数区间
        let lines_per_pixel = std::cmp::max(1, self.total_lines / viewport_height);

        // 统计每个区间的命中数
        for hit_line in &self.hits {
            let pixel_index = ((hit_line as u64) / lines_per_pixel) as usize;
            if pixel_index < counts.len() {
                counts[pixel_index] = counts[pixel_index].saturating_add(1);
            }
        }

        // 找到最大值用于归一化
        let max_count = counts.iter().copied().max().unwrap_or(1);

        // 归一化到 0-255
        counts
            .into_iter()
            .map(|c| {
                if max_count == 0 {
                    0
                } else {
                    ((c as f64 / max_count as f64) * 255.0) as u8
                }
            })
            .collect()
    }

    /// 获取有序迭代器（按行号升序）
    ///
    /// # 返回
    /// - 按升序排列的命中行号迭代器
    pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
        self.hits.iter().map(|n| n as u64)
    }

    /// 获取压缩后的字节大小（用于性能监控）
    ///
    /// # 返回
    /// - 序列化后的字节大小
    pub fn serialized_size(&self) -> usize {
        // RoaringBitmap 的序列化大小
        self.hits.serialized_size()
    }

    /// 序列化为字节数组
    ///
    /// # 返回
    /// - 序列化后的字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.hits.serialized_size());
        self.hits.serialize_into(&mut bytes).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize RoaringBitmap: {}", e);
        });
        bytes
    }

    /// 从字节数组反序列化
    ///
    /// # 参数
    /// - `bytes`: 序列化的字节数组
    /// - `total_lines`: 总行数
    ///
    /// # 返回
    /// - 反序列化后的 SearchIndex（未冻结状态）
    pub fn from_bytes(bytes: &[u8], total_lines: u64) -> Option<Self> {
        RoaringBitmap::deserialize_from(bytes)
            .ok()
            .map(|hits| Self {
                hits,
                sorted_hits: None,
                total_lines,
                frozen: false,
            })
    }

    /// 合并另一个索引（OR 操作）
    ///
    /// # 参数
    /// - `other`: 要合并的另一个索引
    ///
    /// # 注意
    /// 操作后索引会变为未冻结状态。
    pub fn union_with(&mut self, other: &SearchIndex) {
        if self.frozen {
            self.unfreeze();
        }
        self.hits |= &other.hits;
        // 更新总行数为两者的最大值
        self.total_lines = std::cmp::max(self.total_lines, other.total_lines);
    }

    /// 交集另一个索引（AND 操作）
    ///
    /// # 参数
    /// - `other`: 要交集的另一个索引
    ///
    /// # 注意
    /// 操作后索引会变为未冻结状态。
    pub fn intersect_with(&mut self, other: &SearchIndex) {
        if self.frozen {
            self.unfreeze();
        }
        self.hits &= &other.hits;
    }

    /// 差集操作（AND NOT）
    ///
    /// # 参数
    /// - `other`: 要排除的索引
    ///
    /// # 注意
    /// 操作后索引会变为未冻结状态。
    pub fn difference_with(&mut self, other: &SearchIndex) {
        if self.frozen {
            self.unfreeze();
        }
        self.hits -= &other.hits;
    }

    /// 从 RoaringBitmap 创建 SearchIndex
    ///
    /// # 参数
    /// - `hits`: 命中的行号集合
    /// - `total_lines`: 总行数
    ///
    /// # 返回
    /// - 新的 SearchIndex（未冻结状态）
    pub fn from_roaring_bitmap(hits: RoaringBitmap, total_lines: u64) -> Self {
        Self {
            hits,
            sorted_hits: None,
            total_lines,
            frozen: false,
        }
    }

    /// 获取内部 RoaringBitmap 的引用
    pub fn hits(&self) -> &RoaringBitmap {
        &self.hits
    }

    /// 获取内部 RoaringBitmap 的可变引用
    pub fn hits_mut(&mut self) -> &mut RoaringBitmap {
        &mut self.hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_index_is_empty() {
        let index = SearchIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        assert_eq!(index.total_lines(), 0);
    }

    #[test]
    fn test_add_single_hit() {
        let mut index = SearchIndex::new();
        index.add_hit(42);

        assert!(!index.is_empty());
        assert_eq!(index.len(), 1);
        assert!(index.contains(42));
        assert!(!index.contains(41));
        assert!(!index.contains(43));
    }

    #[test]
    fn test_add_multiple_hits() {
        let mut index = SearchIndex::new();
        index.add_hits([5, 10, 15, 20, 25]);

        assert_eq!(index.len(), 5);
        assert!(index.contains(5));
        assert!(index.contains(10));
        assert!(index.contains(15));
        assert!(index.contains(20));
        assert!(index.contains(25));
    }

    #[test]
    fn test_select_operation() {
        let mut index = SearchIndex::new();
        index.add_hits([5, 10, 15, 20, 25]);

        // 测试 select(k) 操作（未冻结状态）
        assert_eq!(index.select(0), Some(5));
        assert_eq!(index.select(1), Some(10));
        assert_eq!(index.select(2), Some(15));
        assert_eq!(index.select(3), Some(20));
        assert_eq!(index.select(4), Some(25));

        // 超出范围返回 None
        assert_eq!(index.select(5), None);
        assert_eq!(index.select(100), None);
    }

    #[test]
    fn test_select_operation_frozen() {
        let mut index = SearchIndex::new();
        index.add_hits([5, 10, 15, 20, 25]);
        index.freeze();

        assert!(index.is_frozen());

        // 测试 select(k) 操作（已冻结状态，O(1) 复杂度）
        assert_eq!(index.select(0), Some(5));
        assert_eq!(index.select(1), Some(10));
        assert_eq!(index.select(2), Some(15));
        assert_eq!(index.select(3), Some(20));
        assert_eq!(index.select(4), Some(25));

        // 超出范围返回 None
        assert_eq!(index.select(5), None);
        assert_eq!(index.select(100), None);
    }

    #[test]
    fn test_select_o1_performance() {
        // 验证 O(1) select 性能
        let mut index = SearchIndex::with_total_lines(1_000_000);

        // 添加 10 万个命中
        for i in (0..1_000_000).step_by(10) {
            index.add_hit(i);
        }

        index.freeze();

        // 在已冻结状态下，select 应该是 O(1)
        let start = std::time::Instant::now();

        // 执行 1000 次 select 操作
        for k in 0..1000 {
            let _ = index.select(k * 100);
        }

        let duration = start.elapsed();

        // 1000 次 O(1) 操作应该在 1ms 内完成
        assert!(
            duration < std::time::Duration::from_millis(1),
            "1000 select operations took {:?}, expected < 1ms",
            duration
        );
    }

    #[test]
    fn test_select_range() {
        let mut index = SearchIndex::new();
        index.add_hits([5, 10, 15, 20, 25, 30, 35]);

        // 获取范围
        let batch = index.select_range(1, 3);
        assert_eq!(batch, vec![10, 15, 20]);

        // 从末尾获取
        let batch = index.select_range(5, 3);
        assert_eq!(batch, vec![30, 35]); // 只有 2 个元素

        // 超出范围
        let batch = index.select_range(10, 3);
        assert!(batch.is_empty());
    }

    #[test]
    fn test_density_map() {
        let mut index = SearchIndex::with_total_lines(1000);
        index.add_hits([100, 300, 500, 700, 900]);

        let density = index.to_density_map(10);
        assert_eq!(density.len(), 10);

        // 检查有命中的像素数量
        let hit_count = density.iter().filter(|&&v| v == 1).count();
        assert!(
            hit_count >= 5,
            "Expected at least 5 hits, got {}",
            hit_count
        );
    }

    #[test]
    fn test_heatmap() {
        let mut index = SearchIndex::with_total_lines(1000);
        // 在第一个区间添加多个命中
        index.add_hits([10, 20, 30, 500, 700]);

        let heatmap = index.to_heatmap(10);
        assert_eq!(heatmap.len(), 10);

        // 第一个区间应该有最高的强度（3个命中）
        assert!(heatmap[0] > 0);
    }

    #[test]
    fn test_density_calculation() {
        let mut index = SearchIndex::with_total_lines(1000);
        index.add_hits([100, 200, 300]);

        let density = index.density();
        assert!((density - 0.003).abs() < 0.0001);
    }

    #[test]
    fn test_serialization() {
        let mut index = SearchIndex::with_total_lines(1000);
        index.add_hits([100, 200, 300, 400, 500]);

        let bytes = index.to_bytes();
        let restored = SearchIndex::from_bytes(&bytes, 1000);

        assert!(restored.is_some());
        let restored = restored.unwrap();

        assert_eq!(restored.len(), 5);
        assert_eq!(restored.total_lines(), 1000);
        assert!(restored.contains(100));
        assert!(restored.contains(300));
        assert!(restored.contains(500));
    }

    #[test]
    fn test_union_operation() {
        let mut index1 = SearchIndex::with_total_lines(1000);
        index1.add_hits([100, 200, 300]);

        let mut index2 = SearchIndex::with_total_lines(1000);
        index2.add_hits([200, 300, 400]);

        index1.union_with(&index2);

        assert_eq!(index1.len(), 4);
        assert!(index1.contains(100));
        assert!(index1.contains(200));
        assert!(index1.contains(300));
        assert!(index1.contains(400));
    }

    #[test]
    fn test_intersect_operation() {
        let mut index1 = SearchIndex::with_total_lines(1000);
        index1.add_hits([100, 200, 300]);

        let mut index2 = SearchIndex::with_total_lines(1000);
        index2.add_hits([200, 300, 400]);

        index1.intersect_with(&index2);

        assert_eq!(index1.len(), 2);
        assert!(index1.contains(200));
        assert!(index1.contains(300));
        assert!(!index1.contains(100));
        assert!(!index1.contains(400));
    }

    #[test]
    fn test_difference_operation() {
        let mut index1 = SearchIndex::with_total_lines(1000);
        index1.add_hits([100, 200, 300]);

        let mut index2 = SearchIndex::with_total_lines(1000);
        index2.add_hits([200, 300, 400]);

        index1.difference_with(&index2);

        assert_eq!(index1.len(), 1);
        assert!(index1.contains(100));
        assert!(!index1.contains(200));
        assert!(!index1.contains(300));
    }

    #[test]
    fn test_iteration_order() {
        let mut index = SearchIndex::new();
        // 以非顺序方式添加
        index.add_hits([30, 10, 50, 20, 40]);

        // 迭代应该是升序的
        let collected: Vec<u64> = index.iter().collect();
        assert_eq!(collected, vec![10, 20, 30, 40, 50]);
    }

    #[test]
    fn test_large_index() {
        // 测试大量数据的压缩效果
        let mut index = SearchIndex::with_total_lines(10_000_000);

        // 添加 100 万个命中（10% 密度）
        for i in (0..10_000_000).step_by(10) {
            index.add_hit(i);
        }

        assert_eq!(index.len(), 1_000_000);

        // 检查压缩后的大小
        let size = index.serialized_size();
        println!(
            "Compressed size for 1M hits: {} bytes ({:.2} MB)",
            size,
            size as f64 / 1024.0 / 1024.0
        );

        // PRD 要求：千万级结果压缩至 < 5MB
        // 这里是 100 万命中，应该 < 500KB
        assert!(size < 5_000_000, "Size {} bytes exceeds 5MB limit", size);

        // 验证 select 操作
        assert_eq!(index.select(0), Some(0));
        assert_eq!(index.select(1), Some(10));
        assert_eq!(index.select(999_999), Some(9_999_990));
        assert_eq!(index.select(1_000_000), None);
    }

    #[test]
    fn test_select_range_performance() {
        // 测试 select_range 在大索引上的性能
        let mut index = SearchIndex::with_total_lines(1_000_000);

        // 添加 10 万个命中
        for i in (0..1_000_000).step_by(10) {
            index.add_hit(i);
        }

        let start = std::time::Instant::now();
        let batch = index.select_range(50_000, 100);
        let duration = start.elapsed();

        assert_eq!(batch.len(), 100);
        // 应该在 1ms 内完成
        assert!(
            duration < std::time::Duration::from_millis(1),
            "select_range took {:?}, expected < 1ms",
            duration
        );
    }

    #[test]
    fn test_empty_operations() {
        let mut index = SearchIndex::new();

        // 空索引的操作
        assert_eq!(index.select(0), None);
        assert!(index.select_range(0, 10).is_empty());
        assert!(index.to_density_map(10).iter().all(|&v| v == 0));

        // 设置总行数后的密度图
        index.set_total_lines(1000);
        let density = index.to_density_map(10);
        assert!(density.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_duplicate_hits() {
        let mut index = SearchIndex::new();

        // 重复添加相同行号
        index.add_hit(42);
        index.add_hit(42);
        index.add_hit(42);

        // 应该只保留一个
        assert_eq!(index.len(), 1);
        assert!(index.contains(42));
    }
}
