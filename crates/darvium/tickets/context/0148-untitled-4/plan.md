# 空間移動力学 — 首長レジストリと引力・斥力による個体移動 — 実装計画

## Requirements
- ChiefRegistry: Arc<RwLock<...>> singleton, synced from Phase 3.8 village_chiefs
- Paramount Chief: max chiefdom_score among chiefs, stationary
- Other Chiefs: attract toward paramount + repel from all other chiefs
- Non-chiefs: move toward nearest chief, switch to 2nd-nearest at MIN_APPROACH_DISTANCE
- Single-chief fallback: move toward random nearby point
- Phase 3.9: inserted after Phase 3.8, before Phase 3.6

## File Changes
| File | Type | Content |
|---|---|---|
| src/constants.rs | Add | MOVEMENT_DISTANCE, MIN_APPROACH_DISTANCE |
| src/chief_registry.rs | NEW | ChiefRegistry, ChiefEntry, methods |
| src/lib.rs | Add | pub mod chief_registry; |
| src/simulation.rs | Add | Phase 3.9, 4 movement functions, Registry integration, T1-T9, O1-O3 |
| web/cube/observation/script.js | Add | Paramount chief visual distinction (optional) |

## Function Design
| Function | File | Japanese translation |
|---|---|---|
| ChiefRegistry::sync_from_chiefs() | chief_registry.rs | 首長マップからレジストリを同期する |
| ChiefRegistry::get_paramount() | chief_registry.rs | 主首長を取得する |
| ChiefRegistry::get_nearest() | chief_registry.rs | 最寄りの首長を取得する |
| ChiefRegistry::get_second_nearest() | chief_registry.rs | 2番目に近い首長を取得する |
| compute_attraction_vector() | simulation.rs | 引力ベクトルを計算する |
| compute_repulsion_vector() | simulation.rs | 斥力ベクトルを計算する |
| move_chiefs() | simulation.rs | 首長を移動する |
| move_non_chiefs() | simulation.rs | 非首長を移動する |
| move_population() | simulation.rs | 人口全体を移動する |

## Implementation Steps
1. constants.rs — MOVEMENT_DISTANCE (0.02), MIN_APPROACH_DISTANCE (0.05)
2. chief_registry.rs — new file with ChiefRegistry, ChiefEntry, methods
3. lib.rs — pub mod chief_registry;
4. simulation.rs — Phase 3.9, movement functions, registry in SimulationContext + tests T1-T9, O1-O3
5. cargo test — regression check
6. Observation test execution + report
7. Frontend: paramount chief visual distinction (optional)

## Review Method
```bash
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" src/chief_registry.rs src/simulation.rs src/constants.rs src/lib.rs | node "$_R/scripts/tickets/review/generate-report.js"
```

Translatability grep: function names (verb), 1-char vars, hardcoded numbers

## Risks
- Zero resultant vector: normalized() division by zero → skip movement
- MIN_APPROACH_DISTANCE oscillation: add ±ε hysteresis if needed
- Arc<RwLock<>> contention: single-owner SimulationContext, synchronous lock acquisition
