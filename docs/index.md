---
layout: home

hero:
  name: Log Analyzer
  text: 把复杂日志，留在本地看清楚
  tagline: 面向开发、测试与运维的大规模日志工作台。导入目录或归档包，快速搜索、组合过滤、沉淀关键词，并持续监听新增日志。
  actions:
    - theme: brand
      text: 5 分钟快速开始
      link: /guide/getting-started
    - theme: alt
      text: 了解系统架构
      link: /architecture/overview

features:
  - icon: ⌁
    title: Offline first
    details: 日志、索引和搜索结果留在本机；无需把敏感排障数据上传到外部服务。
  - icon: ⌕
    title: 为大结果集而设计
    details: 混合查询引擎、批处理执行与磁盘分页协作，避免一次搜索拖垮内存。
  - icon: ◫
    title: 从归档到实时目录
    details: 同一套工作区模型覆盖日志文件、目录、ZIP / TAR / GZ / 7Z 与持续写入场景。
---

<div class="la-section">
  <div class="la-signal">Rust · Tauri 2 · React 19</div>
  <p class="la-eyebrow">One local workspace</p>
  <h2>从导入到定位，保持一条清晰路径</h2>
  <p class="la-section-lead">工作区组织日志源，搜索页把关键词、时间、级别和文件路径组合起来，关键词组则把团队反复使用的故障信号沉淀为可复用资产。</p>
  <div class="la-showcase">
    <figure class="la-shot">
      <img src="./assets/readme/workspaces-overview.png" alt="Log Analyzer 工作区总览，展示多个本地日志工作区及其状态" />
      <figcaption><strong>工作区总览</strong><span>导入、刷新、监听与状态管理</span></figcaption>
    </figure>
    <figure class="la-shot">
      <img src="./assets/readme/search-results.png" alt="Log Analyzer 搜索结果页，展示关键词与过滤后的日志记录" />
      <figcaption><strong>搜索与过滤</strong><span>从大量记录中收敛到高信号结果</span></figcaption>
    </figure>
    <figure class="la-shot">
      <img src="./assets/readme/keyword-groups.png" alt="Log Analyzer 关键词组页面，展示可复用的故障关键词" />
      <figcaption><strong>关键词组</strong><span>复用团队已经验证过的排障线索</span></figcaption>
    </figure>
  </div>
</div>

<div class="la-section">
  <p class="la-eyebrow">Choose your path</p>
  <h2>按你的任务开始阅读</h2>
  <div class="la-path-grid">
    <a class="la-link-card" href="./guide/getting-started">
      <b>USER</b>
      <strong>开始分析日志</strong>
      <span>安装应用、建立工作区，并完成第一次搜索。</span>
    </a>
    <a class="la-link-card" href="./architecture/overview">
      <b>ENGINEER</b>
      <strong>理解数据与调用链</strong>
      <span>查看 Clean Architecture 分层、workspace crates 与关键数据流。</span>
    </a>
    <a class="la-link-card" href="./operations/ci">
      <b>MAINTAINER</b>
      <strong>构建、验证与发布</strong>
      <span>掌握 CI、版本一致性、发布工作流与运行排障。</span>
    </a>
  </div>
</div>

<div class="la-section">
  <p class="la-eyebrow">Data stays local</p>
  <h2>离线优先不是限制，而是边界</h2>
  <p class="la-section-lead">运行时不依赖云端日志平台。CAS 以 SHA-256 去重内容，SQLite 保存元数据，搜索结果按会话分页写入磁盘。明确的数据边界让事故日志、客户现场包和本地联调记录都能在同一工具中处理。</p>
</div>

