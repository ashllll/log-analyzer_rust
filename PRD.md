# 📄 PRD: 纯本地极速日志分析引擎 (The Rustacean Architecture)

**文档版本:** V 6.0 (Rust 语言特性极限压榨版)  
**状态:** Final Blueprint for Development  
**核心技术栈:** * **Backend:** Rust (`memmap2`, `rayon`, `rkyv`, `roaring`, `std::sync::atomic`)
* **Frontend:** Flutter (`SliverFixedExtentList`, `FragmentProgram` GPU 着色)
* **Bridge:** FFI (`flutter_rust_bridge` v2 + 零拷贝内存映射)

---

## 一、 产品愿景与 SLA (Service Level Agreement)

### 1.1 架构核心哲学
超越一切现有文本编辑器的极限。将 Rust 的**“无畏并发 (Fearless Concurrency)”**、**“零成本抽象 (Zero-Cost Abstractions)”**与**“所有权/生命周期 (Ownership & Lifetimes)”**机制深度嵌入系统骨髓，在编译期消灭一切数据竞争与非法状态，在运行期实现纳秒级 (Nanosecond) 的指令吞吐。

### 1.2 工业级 SLA 指标
* **绝对安全:** 编译期保证零数据竞争 (Data Race Free)，运行期保证零 Panic (通过严格的 `Result` 边界封装)。
* **内存复杂度 $O(1)$:** 10GB 单体文件驻留内存 `< 50MB`。千万级搜索结果通过 Roaring Bitmap 压缩至 `< 5MB`。
* **延迟极限:** FFI 提货管道（Viewport Fetching）延迟 `< 1ms`；全量并发盲搜吞吐量触及固态硬盘/内存总线物理带宽极限（约 `3-5GB/s`）。

---

## 二、 核心底层引擎设计 (Rust 语言特性深度赋能)

本节展示如何利用 Rust 独有的语言特性，重构传统 C/C++ 难以安全实现的底层模块。

### 2.1 编译期安全的状态流转 (The Typestate Pattern)
* **痛点:** 传统架构中，文件状态散落，通过 `if is_ready {}` 运行时判断极易引发崩溃。
* **Rust 特性赋能:** **类型状态模式 (Typestate) 与所有权转移**。
  
* **架构设计:** 将会话状态编码进类型系统。
  `Session<Unmapped> -> Session<Mapped> -> Session<Indexed>`
  当调用构建索引方法时，传入的 `Mapped` 状态实例被消费（所有权转移），强迫调用者只能持有最新的 `Indexed` 状态，彻底消灭运行时状态异常。

### 2.2 极致无锁追加索引 (Lock-Free Atomics & Memory Ordering)
* **痛点:** 面对 `tail -f` 疯狂写入，读写内存屏障会引发前端 UI 饥饿。
* **Rust 特性赋能:** **`std::sync::atomic` 与精确内存序 (Acquire/Release Semantics)**。
  
* **架构设计:** 设计基于原子指针的 `Chunked Array` (分块数组，每块如 128KB)。新日志追加时，通过 `compare_exchange(..., Ordering::Release, ...)` 原子挂载到全局树。前端读取使用 `Ordering::Acquire`。真正实现 Wait-Free (无等待)，前后端在物理内存层面完美隔离。

### 2.3 零成本抽象的结构化引擎 (Zero-Cost Traits & Macros)
* **痛点:** 日志格式多变，前端正则高亮性能极差。
* **Rust 特性赋能:** **`Trait` 静态分发 (Static Dispatch) 与 宏 (Macros)**。
* **架构设计:** 定义底层 `LogLexer` Trait。使用过程宏 (Procedural Macros) 声明日志格式。编译器通过单态化 (Monomorphization) 将其内联为 SIMD 机器码，消除虚函数开销。FFI 边界前直接吐出紧凑的 `Binary Token`，废除前端昂贵的正则引擎。

### 2.4 滑动窗口与转码降级 (VMA & Transcoding)
* **机制:** 通过 `PageManager` 维持最多 3GB 的虚拟地址映射，防止 32 位环境 OOM。
* **安全网:** 利用 `chardetng` 探测编码。遭遇 UTF-16 等导致 SIMD 失效的编码，立刻中断 Mmap，退化至流式 UTF-8 临时文件转码管道。

---

## 三、 FFI 网关与极致零拷贝 (The $O(1)$ Memory Bridge)

### 3.1 基于 rkyv 的极端零拷贝穿透 (Extreme Zero-Copy)
* **痛点:** FFI 边界存在从 Rust 堆到 Dart `Uint8List` 的内存复制。
* **Rust 特性赋能:** **`rkyv` (Archive) 零拷贝反序列化库 + 内存固定 (Pinning)**。
  
* **架构设计:** Rust 提取视口数据后，使用 `rkyv` 就地格式化为内存对齐的二进制结构。直接将**原始内存指针 (Raw Pointer)** 暴露给 Dart。Dart 强转为 TypedData 视图结构读取。实现物理意义上的 **0 字节拷贝**。

### 3.2 Roaring Bitmap 拉取模型 (Anti-OOM Pull Model)
* **机制:** 命中数据全部压缩在 Rust 端的 `RoaringBitmap` 内。
* **交互:** Dart 端接收 `total_hits`，根据当前视口，通过 `get_search_highlights` 向后端的 Bitmap 主动发起 $O(1)$ 复杂度的 `$select(k)$` 拉取请求，阻断前端 OOM。

---

## 四、 Flutter 渲染层与 GPU 压榨 (Frontend Mechanics)

### 4.1 强迫症级别的确定性视口 (Strict Deterministic Viewport)
* 强制使用 `SliverFixedExtentList` 结合 `StrutStyle(forceStrutHeight: true)`。彻底镇压多平台/多语言 Fallback 字体造成的行高突变，捍卫 $O(1)$ 视口物理锚点。

### 4.2 GPU 着色器缩略图 (Fragment Shader Minimap)
* **痛点:** CPU 循环绘制千万级热力图会导致严重掉帧。
* **设计:** Rust 将 Bitmap 压缩为低精度 `density_map` (`Uint8List`)。Flutter 通过 `FragmentProgram` 将此数组直接塞给 **GPU 片段着色器 (GLSL)**。
* **成果:** 纳秒级计算滚动条像素颜色热力图，彻底释放主 Isolate CPU。
  

### 4.3 虚拟视图切换 ($O(1)$ Filter View)
* 配合 Roaring Bitmap 的 `$select(k)$` 能力，前端可在毫秒级实现“仅显示包含 ERROR 的行”。无需新建文件，全凭内存寻址魔法完成日志过滤。

---

## 五、 核心 API 契约规范 (Contract Definition V6)

```rust
type SessionId = u64;

// Typestate 标记 (Rust 内部设计，对外透明)
struct Session<S> { id: SessionId, state: S }

// 宏观搜索与 GPU 渲染数据
struct SearchProgress {
    pub query_id: u64,
    pub total_hits: u64,
    pub is_done: bool,
    pub gpu_texture_map: Vec<u8>, // 直接喂给 Flutter Fragment Shader 的纹理数据
}

// 经过 rkyv 处理的零拷贝高亮 Token
#[derive(Archive, Serialize, Deserialize)]
struct HighlightToken {
    pub token_type: u8, 
    pub start_offset: u16,
    pub length: u16,
}



分类,接口定义与 Rust 特性约束
会话,open_session(path) -> SessionId转移至 Session<Mapped> 状态，探测编码。
拉取,"pull_viewport_data(id, start_row, end_row) -> ZeroCopyBuffer通过 rkyv + 裸指针映射，实现 0 次拷贝提取。"
搜索,"execute_search(id, query, query_id) -> Stream<SearchProgress>基于 DFA 与 RoaringBitmap，返回热力图。"
交互,"get_virtual_row(id, bitmap_index) -> u64利用 Bitmap 的 $select(k)$ 实现 O(1) 过滤视图映射。"
守护,watch_rotation(id) -> Stream<u64>基于 std::sync::atomic 无锁 Chunked Array 返回最新行数。

六、 架构演进实施里程碑 (Implementation Strategy)
🚩 M1: Rustacean 底座 (核心难点突破)落地 Typestate 状态机、基于 Atomic 的无锁 Chunked Array 索引树，以及 PageManager 滑动窗口 Mmap。跑通基于 rkyv 零拷贝框架的 FFI 裸指针映射通信。此阶段仅编写 Rust 端 Benchmark 测试，验证极限 I/O 吞吐。
🚩 M2: 渲染剥离与 GPU 融合 (骨架成型)Flutter 侧接入 SliverFixedExtentList 与强制 StrutStyle 约束。编写 GLSL Shader，通过 FFI 获取热力图数组并在 GPU 端完成 Minimap 绘制。
🚩 M3: SIMD 词法与高级检索引擎 (神经元)接入 DFA 正则盲搜与 Roaring Bitmap 压缩位图。使用 Rust 过程宏实现高度优化的 LogLexer 解析器，直接向 Flutter 返回结构化 Token。实现 $O(1)$ 全局过滤模式。
🚩 M4: 工业级容灾与防爆 (最后的长城)补齐非 UTF-8 编码嗅探与降级转码管道。完善无限单行强制虚拟截断机制，防止 JSON 炸弹。接入 Inode 追踪机制，应对外部 logrotate 切割重连。