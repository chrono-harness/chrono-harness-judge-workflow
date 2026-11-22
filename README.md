# chrono-harness-judge-workflow

判断分支、集成证据与退役条件。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/judge-workflow)

## 检查本地分支上下文的有效期

工作树生产者可调用 `chrono-judge-workflow check-observation-age`，通过标准输入提交一份 JSON。

```json
{"context":{"branch_started_at":"2026-01-01T00:00:00Z","observed_at":"2026-01-01T01:00:00Z"},"workflow":{"staleness":{"max_age_hours":1}}}
```

程序只比较调用方给出的 UTC 时间。有效期边界包含等于上限的时刻。有效时退出 0，过期时退出 1，输入或调用有误时退出 2。标准错误保存过期或失败的原因。
