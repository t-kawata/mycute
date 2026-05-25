# Plan: M1.76-7 Child protection integration (F-10)

## Requirements
F-10 `C_i^protect = η_1 · 1[Child(i)] + η_2 · H_i^received + η_3 · G_i^growth` を純粋関数として実装。

## Change files
| File | Type | Content |
|---|---|---|
| src/constants.rs | add | CHILD_PROTECT_ETA1=0.50, ETA2=0.30, ETA3=0.20 |
| src/reciprocity.rs | add | compute_child_protection() function (pub) |
| src/reciprocity.rs mod tests | add | TC-1 to TC-7 |

## Procedure
1. Add constants
2. Implement compute_child_protection
3. Implement TC-1~TC-7
4. cargo test + cargo clippy

## Review
- run-quality-checks.js on both files
- function names as verb phrases
- no hardcoded numeric literals

## Risk
- η defaults may need calibration (M1.76-16)
- Grace Period interaction guaranteed by TC-5
