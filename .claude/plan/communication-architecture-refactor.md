# 前后端通信架构全面重构计划

**项目名称**：日志分析器通信架构重构
**开始时间**：2025-12-14
**预计周期**：12-16周
**目标版本**：v1.0.0

## 重构目标

### 核心目标
1. **性能提升200-300%**：通过微服务化、流式处理、分布式缓存
2. **稳定性提升95%**：消除内存泄漏、panic传播、死锁等致命问题
3. **可扩展性**：支持横向扩展，满足大型企业需求
4. **可观测性**：完整的监控、追踪、告警系统

### 架构升级
- **当前**：单体应用 + Tauri IPC
- **目标**：微服务架构 + 事件驱动 + GraphQL API

## 详细执行计划

### Phase 1：架构设计与基础设施（第1-2周）

#### 任务1.1：微服务架构设计
**文件**：`docs/architecture/microservices.md`
**交付物**：
- 微服务拆分方案
- 服务边界定义
- 数据流设计
- API契约文档

**服务拆分**：
```
├── search-service          # 搜索服务
│   ├── search-engine       # 搜索引擎
│   ├── index-manager       # 索引管理
│   └── query-optimizer     # 查询优化
├── import-service          # 导入服务
│   ├── file-processor      # 文件处理
│   ├── archive-handler     # 压缩包处理
│   └── metadata-extractor  # 元数据提取
├── workspace-service       # 工作区服务
│   ├── workspace-manager   # 工作区管理
│   ├── file-watcher        # 文件监听
│   └── sync-engine         # 同步引擎
├── cache-service           # 缓存服务
│   ├── l1-memory-cache     # 内存缓存
│   ├── l2-redis-cache      # Redis缓存
│   └── cache-invalidator   # 缓存失效
└── api-gateway             # API网关
    ├── authentication      # 认证授权
    ├── rate-limiter        # 限流
    ├── load-balancer       # 负载均衡
    └── request-router      # 请求路由
```

#### 任务1.2：事件驱动架构设计
**文件**：`docs/architecture/event-driven.md`
**交付物**：
- 事件模型定义
- 事件总线设计
- 事件溯源方案
- CQRS模式实现

**核心事件**：
```rust
// 事件定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainEvent {
    // 搜索相关事件
    SearchRequested {
        query_id: String,
        user_id: String,
        query: SearchQuery,
    },
    SearchCompleted {
        query_id: String,
        results_count: usize,
        duration_ms: u64,
    },
    SearchFailed {
        query_id: String,
        error: String,
    },

    // 导入相关事件
    ImportStarted {
        workspace_id: String,
        file_count: usize,
    },
    ImportProgress {
        workspace_id: String,
        processed_files: usize,
        total_files: usize,
    },
    ImportCompleted {
        workspace_id: String,
        total_files: usize,
        total_size: u64,
    },

    // 缓存相关事件
    CacheInvalidated {
        key: String,
        reason: String,
    },
    CacheHit {
        key: String,
        service: String,
    },
    CacheMiss {
        key: String,
        service: String,
    },
}
```

#### 任务1.3：技术栈选型与验证
**文件**：`docs/architecture/tech-stack.md`
**交付物**：
- 技术选型报告
- PoC验证结果
- 性能基准测试

**关键技术**：
- **消息队列**：Apache Kafka（高吞吐）或 NATS（低延迟）
- **缓存**：Redis Cluster + Memcached
- **API**：GraphQL + gRPC（内部通信）
- **数据库**：PostgreSQL（主数据库）+ Elasticsearch（搜索）
- **监控**：Prometheus + Grafana + Jaeger
- **容器化**：Docker + Kubernetes

#### 任务1.4：开发环境搭建
**文件**：`docker-compose.yml`、`k8s/`
**交付物**：
- 本地开发环境
- CI/CD流水线
- 基础设施即代码

### Phase 2：核心服务开发（第3-6周）

#### 任务2.1：Search Service开发
**文件**：`services/search-service/`
**功能**：
- 搜索引擎优化
- 索引管理
- 查询优化
- 流式结果传输

**关键实现**：
```rust
// services/search-service/src/lib.rs
pub struct SearchService {
    index_manager: Arc<IndexManager>,
    query_optimizer: Arc<QueryOptimizer>,
    cache: Arc<dyn Cache<String, SearchResults>>,
    event_bus: EventBus,
}

impl SearchService {
    pub async fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        let span = tracing::span!(Level::INFO, "search.service")
            .with_tag("query.id", &request.query_id)
            .with_tag("workspace.id", &request.workspace_id)
            .start();

        let _guard = span.enter();

        // 1. 查询优化
        let optimized_query = self.query_optimizer.optimize(&request.query).await?;

        // 2. 缓存检查
        if let Some(cached_results) = self.cache.get(&optimized_query.cache_key())? {
            self.event_bus.emit(DomainEvent::CacheHit {
                key: optimized_query.cache_key(),
                service: "search".to_string(),
            });
            return Ok(SearchResponse::from_cache(cached_results));
        }

        // 3. 执行搜索
        let start_time = Instant::now();
        let results = self.index_manager.search(&optimized_query).await?;
        let duration = start_time.elapsed();

        // 4. 流式发送结果
        let response = SearchResponse::new()
            .with_results_stream(results)
            .with_stats(SearchStats {
                duration_ms: duration.as_millis() as u64,
                results_count: results.len(),
            });

        // 5. 缓存结果
        self.cache.put(optimized_query.cache_key(), results.clone())?;

        // 6. 发送事件
        self.event_bus.emit(DomainEvent::SearchCompleted {
            query_id: request.query_id,
            results_count: results.len(),
            duration_ms: duration.as_millis() as u64,
        });

        Ok(response)
    }
}
```

#### 任务2.2：Import Service开发
**文件**：`services/import-service/`
**功能**：
- 大文件处理
- 增量导入
- 并行处理
- 进度追踪

**关键实现**：
```rust
// services/import-service/src/lib.rs
pub struct ImportService {
    file_processor: Arc<FileProcessor>,
    archive_handler: Arc<ArchiveHandler>,
    metadata_extractor: Arc<MetadataExtractor>,
    event_bus: EventBus,
    worker_pool: Arc<WorkerPool>,
}

impl ImportService {
    pub async fn import_workspace(&self, request: ImportRequest) -> Result<ImportResponse> {
        let workspace_id = request.workspace_id.clone();

        // 1. 发送开始事件
        self.event_bus.emit(DomainEvent::ImportStarted {
            workspace_id: workspace_id.clone(),
            file_count: request.files.len(),
        });

        // 2. 分析文件
        let file_groups = self.categorize_files(&request.files);

        // 3. 并行处理不同类型
        let (regular_files, archive_files) = tokio::try_join!(
            self.process_regular_files(workspace_id.clone(), file_groups.regular),
            self.process_archives(workspace_id.clone(), file_groups.archives)
        )?;

        // 4. 合并结果
        let total_files = regular_files.len() + archive_files.len();
        let total_size = regular_files.iter().map(|f| f.size).sum::<u64>()
            + archive_files.iter().map(|f| f.size).sum::<u64>();

        // 5. 发送完成事件
        self.event_bus.emit(DomainEvent::ImportCompleted {
            workspace_id,
            total_files,
            total_size,
        });

        Ok(ImportResponse {
            total_files,
            total_size,
            processed_files: total_files,
        })
    }

    async fn process_regular_files(
        &self,
        workspace_id: String,
        files: Vec<PathBuf>,
    ) -> Result<Vec<FileInfo>> {
        let batch_size = 100;
        let mut results = Vec::new();

        for batch in files.chunks(batch_size) {
            // 并行处理批次
            let handles: Vec<_> = batch
                .iter()
                .map(|file| {
                    self.worker_pool.spawn(async move {
                        let info = self.file_processor.process(file).await?;
                        Ok(info)
                    })
                })
                .collect();

            // 等待批次完成
            for handle in handles {
                let info = handle.await??;
                results.push(info);

                // 发送进度事件
                self.event_bus.emit(DomainEvent::ImportProgress {
                    workspace_id: workspace_id.clone(),
                    processed_files: results.len(),
                    total_files: files.len(),
                });
            }

            // 批次间暂停，避免内存峰值
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(results)
    }
}
```

#### 任务2.3：Cache Service开发
**文件**：`services/cache-service/`
**功能**：
- 多级缓存
- 缓存策略
- 分布式缓存
- 缓存预热

**关键实现**：
```rust
// services/cache-service/src/lib.rs
pub struct CacheService {
    l1_cache: Arc<Mutex<LruCache<String, CachedValue>>>,
    l2_cache: Arc<RedisCache>,
    invalidator: Arc<CacheInvalidator>,
    metrics: Arc<CacheMetrics>,
}

impl CacheService {
    pub async fn get(&self, key: &str) -> Result<Option<CachedValue>> {
        // 1. L1缓存检查
        if let Some(value) = self.l1_cache.lock()?.get(key) {
            self.metrics.record_hit("l1");
            return Ok(Some(value.clone()));
        }

        // 2. L2缓存检查
        if let Some(value) = self.l2_cache.get(key).await? {
            self.metrics.record_hit("l2");
            // 回写到L1
            self.l1_cache.lock()?.put(key.to_string(), value.clone());
            return Ok(Some(value));
        }

        // 3. 缓存未命中
        self.metrics.record_miss();
        Ok(None)
    }

    pub async fn put(&self, key: String, value: CachedValue, ttl: Duration) -> Result<()> {
        // 1. 写入L1
        self.l1_cache.lock()?.put(key.clone(), value.clone());

        // 2. 写入L2
        self.l2_cache.put(&key, &value, ttl).await?;

        // 3. 设置失效定时器
        self.invalidator.schedule_invalidation(key, ttl);

        Ok(())
    }
}
```

#### 任务2.4：API Gateway开发
**文件**：`services/api-gateway/`
**功能**：
- 请求路由
- 认证授权
- 限流熔断
- 负载均衡

**关键实现**：
```rust
// services/api-gateway/src/lib.rs
pub struct ApiGateway {
    router: Router,
    auth_service: Arc<AuthService>,
    rate_limiter: Arc<RateLimiter>,
    load_balancer: Arc<LoadBalancer>,
   熔断器: Arc<CircuitBreaker>,
}

impl ApiGateway {
    pub async fn handle_request(&self, request: Request) -> Result<Response> {
        // 1. 认证
        let user = self.auth_service.authenticate(&request.headers).await?;

        // 2. 限流检查
        if !self.rate_limiter.check_rate_limit(&user.id, &request.path).await? {
            return Err(Error::RateLimitExceeded);
        }

        // 3. 熔断器检查
        if self.熔断器.is_open()? {
            return Err(Error::ServiceUnavailable);
        }

        // 4. 负载均衡选择服务
        let service_instance = self.load_balancer.select_instance(&request.path).await?;

        // 5. 转发请求
        let response = self.forward_request(service_instance, request).await?;

        // 6. 记录指标
        self.metrics.record_request(&request.path, response.status_code);

        Ok(response)
    }
}
```

### Phase 3：前端重构（第7-10周）

#### 任务3.1：GraphQL客户端集成
**文件**：`frontend/src/graphql/`
**功能**：
- GraphQL查询
- 订阅（实时更新）
- 缓存管理
- 错误处理

**关键实现**：
```typescript
// frontend/src/graphql/client.ts
import { ApolloClient, InMemoryCache, HttpLink, from } from '@apollo/client/core';
import { onError } from '@apollo/client/link/error';
import { RetryLink } from '@apollo/client/link/retry';

const errorLink = onError(({ graphQLErrors, networkError, operation, forward }) => {
  if (graphQLErrors) {
    graphQLErrors.forEach(({ message, locations, path }) => {
      console.error(`[GraphQL error]: Message: ${message}, Location: ${locations}, Path: ${path}`);
    });
  }

  if (networkError) {
    console.error(`[Network error]: ${networkError}`);

    // 实现重试逻辑
    if (operation.operationName !== 'healthCheck') {
      return forward(operation);
    }
  }
});

const retryLink = new RetryLink({
  delay: {
    initial: 300,
    max: Infinity,
    jitter: true,
  },
  attempts: {
    max: 5,
    retryIf: (error, _operation) => !!error,
  },
});

export const apolloClient = new ApolloClient({
  link: from([errorLink, retryLink, httpLink]),
  cache: new InMemoryCache({
    typePolicies: {
      Query: {
        fields: {
          searchResults: {
            keyArgs: ['query', 'workspaceId'],
            merge(existing = [], incoming) {
              return [...existing, ...incoming];
            },
          },
        },
      },
    },
  }),
  defaultOptions: {
    watchQuery: {
      errorPolicy: 'all',
    },
  },
});
```

#### 任务3.2：实时订阅实现
**文件**：`frontend/src/hooks/useRealtime.ts`
**功能**：
- WebSocket连接管理
- 自动重连
- 心跳检测
- 连接状态监控

**关键实现**：
```typescript
// frontend/src/hooks/useRealtime.ts
export function useRealtime() {
  const [connectionState, setConnectionState] = useState<ConnectionState>('DISCONNECTED');
  const [reconnectAttempts, setReconnectAttempts] = useState(0);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout>();

  const connect = useCallback(() => {
    const ws = new WebSocket('wss://api.log-analyzer.com/graphql');

    ws.onopen = () => {
      setConnectionState('CONNECTED');
      setReconnectAttempts(0);

      // 发送心跳
      const heartbeat = setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ type: 'ping' }));
        }
      }, 30000);

      wsRef.current = ws;
      wsRef.current.heartbeat = heartbeat;
    };

    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);

      if (data.type === 'pong') {
        return; // 心跳响应
      }

      if (data.type === 'subscription_data') {
        handleSubscriptionData(data);
      }
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    ws.onclose = () => {
      setConnectionState('DISCONNECTED');

      // 清理心跳
      if (wsRef.current?.heartbeat) {
        clearInterval(wsRef.current.heartbeat);
      }

      // 自动重连
      if (reconnectAttempts < 5) {
        const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
        reconnectTimeoutRef.current = setTimeout(() => {
          setReconnectAttempts(prev => prev + 1);
          connect();
        }, delay);
      }
    };

    return ws;
  }, [reconnectAttempts]);

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
    }

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setConnectionState('DISCONNECTED');
  }, []);

  useEffect(() => {
    const ws = connect();
    return () => disconnect();
  }, []);

  return {
    connectionState,
    reconnectAttempts,
    connect,
    disconnect,
  };
}
```

#### 任务3.3：搜索页面优化
**文件**：`frontend/src/pages/SearchPage.tsx`
**改进**：
- 流式结果显示
- 虚拟滚动优化
- 增量加载
- 智能缓存

#### 任务3.4：任务管理优化
**文件**：`frontend/src/hooks/useTaskManager.ts`
**改进**：
- 实时进度更新
- 任务取消
- 错误恢复
- 状态持久化

### Phase 4：性能优化（第11-12周）

#### 任务4.1：流式处理优化
**实现目标**：
- 大文件支持（>1GB）
- 内存使用降低60%
- 响应时间减少70%

**关键实现**：
```rust
// 流式搜索结果
#[graphqlSubscription]
async fn search_results_stream(
    &self,
    query: String,
    workspace_id: ID,
) -> impl Stream<Item = SearchResultChunk> {
    let mut index_reader = self.index_manager.get_reader(&?;

    letworkspace_id).await stream = async_stream::stream! {
        let mut batch = Vec::new();
        let mut line_number = 0;

        while let Some(line) = index_reader.next_line().await? {
            line_number += 1;

            if self.query.matches(&line.content) {
                batch.push(SearchResult {
                    id: format!("{}-{}", line.file_path, line_number),
                    content: line.content,
                    file_path: line.file_path,
                    line_number,
                    matches: line.matches,
                });

                // 批量发送
                if batch.len() >= 100 {
                    yield SearchResultChunk {
                        results: batch.clone(),
                        has_more: true,
                    };
                    batch.clear();

                    // 避免阻塞事件循环
                    tokio::task::yield_now().await;
                }
            }

            // 定期发送进度
            if line_number % 10000 == 0 {
                yield SearchResultChunk {
                    results: vec![],
                    has_more: true,
                    progress: Some(SearchProgress {
                        processed_lines: line_number,
                        estimated_total: index_reader.total_lines(),
                    }),
                };
            }
        }

        // 发送剩余结果
        if !batch.is_empty() {
            yield SearchResultChunk {
                results: batch,
                has_more: false,
            };
        }
    };

    stream
}
```

#### 任务4.2：缓存优化
**实现目标**：
- 缓存命中率>85%
- 缓存延迟<5ms
- 内存使用优化50%

**关键实现**：
```rust
// 智能缓存预热
struct CacheWarmer {
    cache: Arc<CacheService>,
    query_predictor: Arc<QueryPredictor>,
    access_tracker: Arc<AccessTracker>,
}

impl CacheWarmer {
    pub async fn warm_up(&self, workspace_id: &str) -> Result<()> {
        // 1. 分析历史访问模式
        let popular_queries = self.query_predictor.predict_popular_queries(workspace_id, 100).await?;

        // 2. 并行预热
        let handles: Vec<_> = popular_queries
            .into_iter()
            .map(|query| {
                tokio::spawn(async move {
                    let results = execute_search(&query).await?;
                    Ok((query.cache_key(), results))
                })
            })
            .collect();

        // 3. 等待所有预热完成
        for handle in handles {
            if let Ok(Ok((key, results))) = handle.await {
                self.cache.put(key, results, Duration::from_hours(1)).await?;
            }
        }

        Ok(())
    }
}
```

#### 任务4.3：数据库优化
**实现目标**：
- 查询性能提升90%
- 连接池利用率>80%
- 死锁风险消除

**关键实现**：
```rust
// 连接池管理
struct DatabasePool {
    read_pool: Arc<Pool<Postgres>>,
    write_pool: Arc<Pool<Postgres>>,
    pool_config: PoolConfig,
}

impl DatabasePool {
    pub async fn execute_read<T>(&self, query: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = self.read_pool.get().await?;
        let result = conn.query_one(query, &[]).await?;
        Ok(result.try_get::<_, T>(0)?)
    }

    pub async fn execute_write<T>(&self, query: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = self.write_pool.get().await?;
        let result = conn.query_one(query, &[]).await?;
        Ok(result.try_get::<_, T>(0)?)
    }
}
```

### Phase 5：测试与部署（第13-14周）

#### 任务5.1：集成测试
**文件**：`tests/integration/`
**测试场景**：
- 端到端搜索流程
- 大文件导入流程
- 并发请求处理
- 故障恢复测试

#### 任务5.2：性能测试
**文件**：`tests/performance/`
**测试指标**：
- 搜索吞吐量：>10,000 QPS
- 导入速度：>1GB/min
- 内存使用：<500MB
- 响应延迟：P95 < 100ms

#### 任务5.3：压力测试
**文件**：`tests/stress/`
**测试场景**：
- 1000并发用户
- 10GB数据量
- 72小时稳定运行
- 自动故障恢复

#### 任务5.4：灰度发布
**部署策略**：
1. 内部环境部署（Week 13）
2. 5%流量灰度（Week 14）
3. 50%流量灰度（Week 15）
4. 100%流量切换（Week 16）

### Phase 6：监控与可观测性（第15-16周）

#### 任务6.1：监控体系
**工具**：Prometheus + Grafana
**监控指标**：
- 服务健康状态
- 性能指标
- 业务指标
- 错误率统计

#### 任务6.2：链路追踪
**工具**：Jaeger
**追踪内容**：
- 请求全链路追踪
- 性能瓶颈定位
- 错误根因分析

#### 任务6.3：告警系统
**工具**：AlertManager
**告警规则**：
- 服务不可用
- 性能指标异常
- 错误率超阈值
- 资源使用过高

#### 任务6.4：日志系统
**工具**：ELK Stack
**日志内容**：
- 访问日志
- 错误日志
- 审计日志
- 业务日志

## 风险评估与缓解措施

### 高风险项

#### 风险1：数据迁移风险
**风险描述**：从单体架构迁移到微服务可能导致数据不一致
**缓解措施**：
- 双写模式过渡
- 数据一致性校验
- 回滚机制准备
- 充分测试验证

#### 风险2：性能下降
**风险描述**：微服务架构可能引入额外的网络开销
**缓解措施**：
- 性能基准测试
- 优化服务间通信
- 批量处理优化
- 连接池管理

#### 风险3：复杂性增加
**风险描述**：微服务架构增加系统复杂性
**缓解措施**：
- 完善的文档
- 自动化运维
- 监控系统覆盖
- 团队培训

### 中风险项

#### 风险4：技术选型错误
**风险描述**：新技术栈可能不成熟或不适合
**缓解措施**：
- PoC验证
- 技术调研
- 专家咨询
- 备选方案

#### 风险5：团队技能不足
**风险描述**：团队可能不熟悉新技术
**缓解措施**：
- 提前培训
- 技术分享
- 外部专家指导
- 分阶段实施

## 成功标准

### 性能指标
- [ ] 搜索性能提升200%
- [ ] 大文件处理速度提升300%
- [ ] 内存使用降低60%
- [ ] 响应时间P95 < 100ms

### 稳定性指标
- [ ] 系统可用性 > 99.9%
- [ ] 错误率 < 0.1%
- [ ] 自动恢复时间 < 30s
- [ ] 零内存泄漏

### 可维护性指标
- [ ] 代码覆盖率 > 90%
- [ ] 文档完整性 100%
- [ ] 监控覆盖率 100%
- [ ] 自动化测试覆盖 100%

### 用户体验指标
- [ ] 页面加载时间 < 2s
- [ ] 搜索结果展示 < 500ms
- [ ] 实时进度反馈
- [ ] 错误信息清晰

## 资源需求

### 人力资源
- **架构师**：1人，全程参与
- **后端开发**：3-4人
- **前端开发**：2-3人
- **测试工程师**：2人
- **DevOps工程师**：1人
- **总计**：9-11人

### 时间资源
- **总工期**：16周（4个月）
- **关键路径**：服务开发 → 前端重构 → 测试部署
- **并行任务**：文档编写、测试开发、环境搭建

### 技术资源
- **服务器**：20台（生产环境）
- **开发环境**：云服务器集群
- **测试环境**：独立部署环境
- **监控工具**：Prometheus + Grafana + Jaeger

## 后续规划

### v1.1 计划（3个月后）
- 多租户支持
- 更丰富的图表分析
- 自定义仪表板
- 高级权限管理

### v1.2 计划（6个月后）
- 机器学习日志分析
- 异常检测算法
- 智能告警
- 自然语言查询

### v2.0 规划（1年后）
- 云原生架构
- 多云支持
- 边缘计算
- AI驱动分析

---

## 附录

### A. 技术文档清单
- [ ] 架构设计文档
- [ ] API接口文档
- [ ] 数据库设计文档
- [ ] 部署运维手册
- [ ] 故障处理手册

### B. 代码仓库结构
```
log-analyzer/
├── services/
│   ├── search-service/
│   ├── import-service/
│   ├── workspace-service/
│   ├── cache-service/
│   └── api-gateway/
├── frontend/
│   ├── src/
│   └── public/
├── infrastructure/
│   ├── k8s/
│   ├── terraform/
│   └── docker/
├── docs/
│   ├── architecture/
│   ├── api/
│   └── deployment/
└── tests/
    ├── unit/
    ├── integration/
    ├── performance/
    └── stress/
```

### C. 关键决策记录
- [ ] 微服务拆分决策
- [ ] 技术栈选型
- [ ] 数据存储方案
- [ ] 缓存策略
- [ ] 监控方案

---

**计划创建时间**：2025-12-14
**计划版本**：v1.0
**计划状态**：待审批
