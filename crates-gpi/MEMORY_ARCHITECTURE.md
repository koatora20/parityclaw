# ParityClaw メモリアーキテクチャ設計書

> Decision: 案B — ブリッジ共存 (2026-03-19)

## 原則

- **デフォルト = OpenCrabs Memory** (qmd, FTS5+Vector RRF)
- **オプション = Eternal Memory** (guava-anti, L0-L6, クラウド同期)
- **upstream 追従を壊さない** — `src/memory/` は一切いじらない

## アーキテクチャ

```
┌─────────────────────────────────────────────┐
│  ParityClaw (OpenCrabs fork)                │
│                                             │
│  ┌───────────────┐  ┌────────────────────┐  │
│  │ memory_search │  │ gpi_memory (bridge)│  │
│  │ (built-in)    │  │ (crates-gpi/)      │  │
│  └───────┬───────┘  └────────┬───────────┘  │
│          │                   │              │
│          ▼                   ▼              │
│  ┌───────────────┐  ┌────────────────────┐  │
│  │ ~/.opencrabs/ │  │ guava-anti binary  │  │
│  │ memory/       │  │ (stdio subprocess) │  │
│  │ memory.db     │  └────────┬───────────┘  │
│  │               │           │              │
│  │ qmd Store     │           ▼              │
│  │ FTS5 + Vec    │  ┌────────────────────┐  │
│  │ 768dim        │  │ guava.sqlite       │  │
│  │ RRF hybrid    │  │ L0-L6 layers       │  │
│  └───────────────┘  │ 384/768dim vector  │  │
│                     │ Cloudflare D1 sync │  │
│   DEFAULT ★         └────────────────────┘  │
│                      OPTIONAL (--eternal)   │
└─────────────────────────────────────────────┘
```

## ツール名マッピング

| Tool | Source | Default? | Description |
|------|--------|----------|-------------|
| `memory_search` | OpenCrabs built-in | ✅ YES | セッション短期記憶 (RRF) |
| `gpi_memory` | GuavaBridge → Eternal | ❌ Optional | 長期記憶 + クラウド同期 |

## LLM から見た使い分け

```
User: "さっきの会話で何を話した？"
→ memory_search (短期、セッション内)

User: "先週の研究結果を教えて"
→ gpi_memory search (長期、クロスセッション)

User: "この発見を永久に覚えておいて"
→ gpi_memory store (Eternal Memory に永続保存)
```

## upstream 追従への影響

- **コンフリクト: ゼロ** — `src/memory/` を触らない
- OpenCrabs が memory アップデート → そのまま取り込み
- GPI 独自機能は `crates-gpi/guava-bridge/` に隔離
