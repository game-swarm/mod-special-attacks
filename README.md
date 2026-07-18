# special-attacks

特殊攻击模组。管理 8 种特殊 action 的状态机。

## 职责

- 按 `SpecialAttacksConfig` 的模式 allowlist 注册特殊 action handler 到 ActionRegistry
- Status buffer 系统（S16-S22b），每类 special action 独立 typed buffer：

| Action | Buffer | ID | 效果 |
|--------|--------|-----|-------|
| Hack | HackBuffer | S16 | 夺取敌方 drone 控制权（短时） |
| Drain | DrainBuffer | S17 | 每 tick 吸取目标 Energy |
| Overload | OverloadBuffer | S18 | 增加目标冷却/疲劳 |
| Debilitate | DebilitateBuffer | S19 | 降低目标属性 |
| Disrupt | DisruptBuffer | S20 | 中断持续型动作 |
| Fortify | FortifyBuffer | S21 | 增强己方防御/抗性 |
| Leech | LeechBuffer | S22a | 攻击回血 |
| Fabricate | FabricateBuffer | S22b | 消耗资源生成临时实体 |

- [S22] `status_advance_system` — 唯一 StatusState writer，从 S14 reducer + S16-S22b buffers 读取并统一推进
- 特殊攻击与 HP 伤害互斥——同一 body part 同一 tick 只能执行一种
- 持续型攻击在 drone 移动或被 Disrupt 时中断
- `damage_multiplier` 和 action allowlist 由 strict mod control plane 注入 `SpecialAttacksConfig`

## 依赖

- bevy

## 配置

Engine 按 `mod.toml` 类型定义解码全局开关、世界 allowlist 和 fixed-bp 伤害倍率，并在注册 action handler 前按世界模式选择对应集合：
```toml
[config]
special_attacks_enabled = { type = "bool", default = true }
enabled = { type = "array<SpecialAttack>", default = ["Hack", "Drain", "Overload", "Debilitate", "Disrupt", "Fortify", "Leech", "Fabricate"] }
tutorial_enabled = { type = "array<SpecialAttack>", default = ["Hack", "Drain", "Fortify"] }
novice_enabled = { type = "array<SpecialAttack>", default = ["Hack", "Drain", "Overload", "Fortify"] }
damage_multiplier = { type = "fixed_bp", default = 10000 }
```

Tutorial 使用 `tutorial_enabled`，Novice 使用 `novice_enabled`，其他模式使用 `enabled`；`special_attacks_enabled = false` 时不注册特殊攻击。

## 事件

- 读取: `ActionRegistry`, `PendingDamage`, `PendingHeal`
- 写入: `HackBuffer`, `DrainBuffer`, `OverloadBuffer`, `DebilitateBuffer`, `DisruptBuffer`, `FortifyBuffer`, `LeechBuffer`, `FabricateBuffer`, `StatusState`

## Standalone Development

This repository is consumable as an independent Cargo crate. Its `swarm-engine` dependency is pinned in `Cargo.toml`, so no sibling checkout layout is required.

```sh
cargo check
cargo test
```
