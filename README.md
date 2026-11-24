# chrono-harness-judge-workflow

判断分支、集成证据与退役条件。

在仓库根目录运行 `mise exec -- ./check.sh`。本仓的 mise 配置固定 Verus 版本，检查按变更验证分支、执行和证书及其消费者。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/judge-workflow)

## 检查本地分支上下文的有效期

工作树生产者可调用 `chrono-judge-workflow check-observation-age`，通过标准输入提交一份 JSON。

```json
{"context":{"branch_started_at":"2026-01-01T00:00:00Z","observed_at":"2026-01-01T01:00:00Z"},"workflow":{"staleness":{"max_age_hours":1}}}
```

程序只比较调用方给出的 UTC 时间。有效期边界包含等于上限的时刻。有效时退出 0，过期时退出 1，输入或调用有误时退出 2。标准错误保存过期或失败的原因。

完整工作流通过项目判官的公开命令解释执行结果和验证收集证据。宿主从 chrono-harness-judge-projects 安装支持 `read-results-v1` 和 `check-collection-v1` 的发行。宿主可以在 tools 登记内用 ID `chrono-judge-projects` 指定 program。使用默认项目判官的宿主直接复用其登记的可执行路径和摘要。两项登记都缺席时，查询使用登记的执行环境 PATH。普通结果和历史凭证均以项目判官输出的规范化结果计算摘要。单独的分支有效期查询可直接运行。

workflow 使用宿主固定的 `chrono-judge-routes` 核对原始执行计划和操作回执。集成证据与交付复核都使用同一发行的 `check-plan-receipts-v1`、`plan-context-v1` 和 `collection-plan-v1`。计划顺序、回执匹配和参数展开由 routes 解释。workflow 核对交付上下文、测试完成情况、收集义务和原始报告，再产生工作流判定。

完整工作流还调用已固定的 registration 发行中的 `workflow-views-v1`。两个端点的原始登记从宿主指定提交读取。registration 解释登记、输入和退役声明，workflow 使用实际执行结果、旧成本和原始报告判定集成与交付。

本仓的生产清单、测试清单和锁文件仅引用本仓与第三方 Rust 包。完整判官通过 PATH 调用宿主安装的 `chrono-harness` 读取固定 Git 事实、核对封存请求和原进程回执，并在交付复核期间保护原件。分支有效期查询仍可单独使用。两个提交端点、注册的工具和 SHA-256、证书中的结果摘要及各判官的原始进程继续参与集成与交付判定。
