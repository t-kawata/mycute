# Darvium RFC-0001 Unified Edition v2.3-f 改訂指示書

## 目的

本指示書は、Darvium RFC-0001 Unified Edition v2.3-e を基底として、**直接互恵性 (Direct Reciprocity)** と **間接互恵性 (Indirect Reciprocity)** が workflow の生存確率、支援優先度、成熟促進、淘汰抑制に系統的に影響するように、v2.3-f で追加・補強すべき理論、数式、データ構造、アルゴリズム、較正ループ、実験、監査、マイルストーンを、RFC 改訂担当者向けに極めて具体的に記述するための改訂指示書である。[file:24]

本改訂の中心思想は明確である。**Darvium の宇宙は優しい世界であり、他者に対して協力的で、助け合いに貢献し、評判の良い workflow ほど長く生き残り、より選ばれ、より育ちやすいべきである。** ただしその実装は、既存の ApplicabilityScore、TrustProfile、ReputationProfile、LifecycleScore、GcState、Training Plane、SearchTrace、TrainingRunLog、deterministic replay、property-based test、calibration candidate discipline を毀損せず、strictly additive に統合されなければならない。[file:24]

v2.3-e はすでに、Lifecycle / GC・互恵性ベース評判・Grace Period・VirtualClock・ReputationProfile・HELP 5 段階・child / adult / local village・stability / dynamicity・calibration candidate・replay / perturbation / property-based test という重要な足場を提供している。[file:24] v2.3-f では、その足場の上に、**「優しさが生存確率を上げる」ことを数理的に保証する family of equations と、実装で安全に較正できる calibration loop** を追加する。

---

## 改訂の基本方針

### 1. strictly additive を維持すること

本改訂は、v2.3-e の責務境界・所有権境界・状態機械・Training / Production separation・dual-store consistency・SearchState legality・promotion / repair invariants を変更してはならない (MUST NOT)。[file:24]

特に次は変更対象ではなく、**組み込み先**である。

- `TrustProfile` の 4 軸構成。[file:24]
- `ReputationProfile` の `direct_score / indirect_score / experience_score / inherited_score / final_score`。[file:24]
- `LifecycleScore L(G)` が GC 判定に使われるという既存原則。[file:24]
- `GcState` と Grace Period による保護。[file:24]
- HELP extension における child / adult / local village / helper weighting / bounded remote exploration。[file:24]
- ranking stability / oscillation risk に対する replay / property-based test / calibration discipline。[file:24]

### 2. normative family + calibrated coefficients

RFC 本文には、係数を固定し切った最終式だけを書くのではなく、**単調性制約を持つ関数 family** を規範として書くべきである。係数は `Calibration Candidate` または `EnvironmentPolicy` で管理し、silent drift を禁止する。[file:24]

すなわち、RFC では次を分離して書く。

- **規範 (normative):** 協力・互恵・評判の増加が survival 側に正寄与し、淘汰ハザードに負寄与すること。
- **較正対象 (calibration candidate):** 寄与の強さ、時間減衰、飽和速度、child 保護の強さ、adult maturity threshold、remote exploration rate など。[file:24]

### 3. 「優しさ」を selection と survival の両方へ入れる

v2.3-f では、互恵性は単に reputation の見た目指標ではなく、少なくとも次の 4 箇所に効かなければならない。

1. `ReputationProfile.final_score` の再計算。[file:24]
2. `LifecycleScore` の構成、または LifecycleScore から導出される GC hazard の計算。[file:24]
3. village / HELP proposal における helper weighting。[file:24]
4. child growth / maturation speed、または support success probability。[file:24]

---

## RFC 本文に追加すべき設計目的

### 追加目的文の案

Lifecycle / Natural Selection / GC 節または 41B の補足冒頭に、次の主旨を文章として追加すること。

- Darvium は単なる性能淘汰系ではなく、**協力的な ecosystem を選好する人工生態系**である。[file:24]
- workflow の生存は、成功率・鮮度・使用度のみならず、**他者への貢献・直接互恵・間接互恵・支援実績・優しさの評判**に依存しなければならない。[file:24]
- child support village における HELP 成功、他者の成熟促進、支援の受諾率、裏切りの少なさは、将来の再利用・評判・生存保護へ接続されるべきである。[file:24]
- RFC の normative intent は、**benevolent cooperation is evolutionarily rewarded** である。

この設計目的は理念文で終わらせず、以下の数式で operational に接続すること。

---

## 組み込むべき数理モデル

## 1. Reciprocity contribution の分解

v2.3-e にはすでに `ReputationProfile.direct_score` と `indirect_score` があるため、v2.3-f ではこれを単なる保存フィールドではなく、**再計算規則を持つ意味論的指標**として定義する。[file:24]

### 1.1 Direct Reciprocity

workflow \(i\) の直接互恵性スコアを次で定義する。

\[
R_i^{\mathrm{dir}} = \sigma\left(
\sum_{j \neq i}
\omega_{ij}^{\mathrm{dir}}
\left(
\alpha_h H_{ij}
+ \alpha_{hs} HS_{ij}
- \alpha_r RJ_{ij}
- \alpha_d DMG_{ij}
\right)
\exp(-\rho_{dir} \Delta t_{ij})
\right) \tag{F-1}
\]

ここで:

- \(H_{ij}\): workflow \(i\) が \(j\) に対して help offer / execution を行った回数または強度。
- \(HS_{ij}\): その支援が HelpSuccess に至った回数または強度。[file:24]
- \(RJ_{ij}\): 一度 accepted した支援を途中で破綻させた、または期待された協力を返さなかった回数。
- \(DMG_{ij}\): 他者に負担や失敗を押し付けた harmful interaction の強度。
- \(\Delta t_{ij}\): 最終相互作用からの Human Time または Virtual Time に基づく経過量。[file:24]
- \(\rho_{dir}\): 直接互恵性の時間減衰係数。
- \(\sigma\): 値域を \([0,1]\) に押し込む logistic または calibrated sigmoid。

**重要な normative constraint**:

- \(\alpha_h, \alpha_{hs} > 0\)
- \(\alpha_r, \alpha_d > 0\)
- 協力行為は \(R_i^{\mathrm{dir}}\) を増加させ、裏切り・害は減少させなければならない (MUST)。

### 1.2 Indirect Reciprocity

workflow \(i\) の間接互恵性スコアは、全体ネットワークから見た「この workflow は他者を助ける存在である」という第三者評価を表す。v2.3-f では、HELP network 上の global benevolence として次を推奨する。

\[
R_i^{\mathrm{ind}} = \sigma\left(
\beta_1 C_i^{\mathrm{help}}
+ \beta_2 A_i^{\mathrm{village}}
+ \beta_3 U_i^{\mathrm{accepted}}
+ \beta_4 Q_i^{\mathrm{success}}
- \beta_5 B_i^{\mathrm{harm}}
\right) \tag{F-2}
\]

ここで:

- \(C_i^{\mathrm{help}}\): helper network 上の中心性。PageRank、eigenvector centrality、または weighted in/out degree を採用してよい。[file:24]
- \(A_i^{\mathrm{village}}\): local village 内で child support に安定参加した回数・重み。
- \(U_i^{\mathrm{accepted}}\): offer が child に accept された率。
- \(Q_i^{\mathrm{success}}\): 実支援が child の mission success に寄与した率。
- \(B_i^{\mathrm{harm}}\): rejection / abandonment / harmful mismatch による負評価。

**意図**: direct reciprocity は「相手と自分の関係」、indirect reciprocity は「社会全体から見た善良さ」を表す。v2.3-f では両者を分離したまま保持し、最終評判へ統合する。[file:24]

### 1.3 Benevolence aggregate

RFC には、互恵性と評判と優しさの合成量として `BenevolenceScore` を追加してよい。推奨式は次のとおり。

\[
B_i = w_{dir} R_i^{\mathrm{dir}} + w_{ind} R_i^{\mathrm{ind}} + w_{rep} \operatorname{Rep}_i \tag{F-3}
\]

ここで \(\operatorname{Rep}_i\) は `ReputationProfile.final_score` である。[file:24]

`BenevolenceScore` を独立フィールドとして保存してもよいが、保存しない場合でも SearchTrace / Lifecycle recompute / TrainingRunLog の中間値として再現可能でなければならない (SHOULD)。[file:24]

---

## 2. ReputationProfile の再定義

v2.3-e には `ReputationProfile` の型があるが、v2.3-f では final score の意味をより明示する。[file:24]

### 2.1 Final reputation recompute

\[
\operatorname{Rep}_i = \operatorname{clip}_{[0,1]}\Big(
\theta_{dir} R_i^{\mathrm{dir}}
+ \theta_{ind} R_i^{\mathrm{ind}}
+ \theta_{exp} E_i^{\mathrm{norm}}
+ \theta_{inh} I_i
\Big) \tag{F-4}
\]

ここで:

- \(E_i^{\mathrm{norm}}\): `experience_count` を飽和正規化した値。
- \(I_i\): inherited score。[file:24]
- 係数は非負であり、\(\theta_{dir} + \theta_{ind} + \theta_{exp} + \theta_{inh} = 1\) を推奨する。

**必須制約**:

- `direct_score` と `indirect_score` の寄与は 0 であってはならない (MUST NOT) unless environment policy が明示的に village-help を無効化している場合。
- `final_score` は direct / indirect reciprocity が増加したとき、他条件一定なら非減少でなければならない (MUST)。

### 2.2 Experience 正規化

経験値が評判を無限に押し上げるのを防ぐため、次のような飽和変換を推奨する。

\[
E_i^{\mathrm{norm}} = 1 - \exp(-\kappa_E \cdot \operatorname{experiencecount}(i)) \tag{F-5}
\]

これにより、古参が有利すぎる格差固定化を緩和できる。これは v2.3-e の inherited reputation と experience score の tuning intent に整合する。[file:24]

---

## 3. 生存確率・淘汰ハザードへの組み込み

## 3.1 LifecycleScore を benevolence-aware に拡張する方法

既存 `LifecycleScore L(G)` は「時間鮮度・成功率・trust・使用度・評判」を統合した生存スコアと定義されている。[file:24] v2.3-f では、ここでいう評判を単なる静的 reputation ではなく、**mutual aid driven reputation** として operationalize する。

RFC への記載方法は 2 通りある。

### 推奨案 A: LifecycleScore の構成要素として benevolence を追加

\[
L_i = w_f F_i + w_s S_i + w_t T_i + w_u U_i + w_r \operatorname{Rep}_i + w_b B_i \tag{F-6}
\]

- \(F_i\): freshness / time decay factor。[file:24]
- \(S_i\): success factor。
- \(T_i\): trust composite。
- \(U_i\): usage / reuse factor。
- \(\operatorname{Rep}_i\): final reputation。
- \(B_i\): benevolence aggregate。

この場合、`LIFECYCLE_WEIGHT_*` に `LIFECYCLE_WEIGHT_BENEVOLENCE` を追加する。

### 推奨案 B: LifecycleScore から GC hazard を導く段で benevolence を追加

既存 LifecycleScore はそのまま残し、GC hazard 側で benevolence を効かせる。

\[
\lambda_i^{GC} = \operatorname{softplus}\left(
\lambda_0
- \gamma_L L_i
- \gamma_B B_i
- \gamma_C C_i^{protect}
\right) \tag{F-7}
\]

ここで:

- \(\lambda_i^{GC}\): workflow \(i\) の淘汰ハザード。
- \(C_i^{protect}\): child protection / grace / support-protected term。
- `softplus` を使うことで常に非負。[file:24]

GC 判定に使う離散確率は次でよい。

\[
p_{GC}(i;\Delta t) = 1 - \exp(-\lambda_i^{GC} \Delta t) \tag{F-8}
\]

**v2.3-f で必ず書くべき規範**:

- \(\frac{\partial \lambda_i^{GC}}{\partial R_i^{dir}} \le 0\)
- \(\frac{\partial \lambda_i^{GC}}{\partial R_i^{ind}} \le 0\)
- \(\frac{\partial \lambda_i^{GC}}{\partial \operatorname{Rep}_i} \le 0\)

すなわち、優しさと評判は淘汰ハザードを上げてはならない (MUST NOT)。

### 3.2 survival probability の明示

読者に分かりやすくするため、「優しい workflow ほど生存確率が高い」を直接式にして書くことを推奨する。

\[
P_{survive}(i;\Delta t)=\exp(-\lambda_i^{GC}\Delta t) \tag{F-9}
\]

このとき \(R_i^{dir}, R_i^{ind}, \operatorname{Rep}_i\) の増加は \(P_{survive}\) を非減少にしなければならない。これが Darvium の「優しい宇宙」の最も直接的な数理表現である。

### 3.3 child 保護との接続

Grace Period はすでに `experience_count < MIN_SURVIVAL_EXPERIENCE` の child を GC から保護している。[file:24] v2.3-f ではこれを弱めず、むしろ benevolence を child 成長に接続する。

推奨する child protection 項は次である。

\[
C_i^{protect} = \eta_1 \mathbf{1}[\operatorname{Child}(i)] + \eta_2 H_i^{received} + \eta_3 G_i^{growth} \tag{F-10}
\]

- \(H_i^{received}\): child として有効支援を受けた量。
- \(G_i^{growth}\): child が maturation に向けて改善している量。

これにより「今は弱いが、助けられ、育っている child」は消されにくくなる。

---

## 4. HELP / village における組み込み

v2.3-e の 41B には child / adult / local village / helper proposal / weighting の導入がある。[file:24] v2.3-f では、この weighting に benevolence を明示的に入れる。

### 4.1 helper weighting

既存の helper quality score \(Q(h,c,M)\) に対し、v2.3-f では次を推奨式として追加する。

\[
Q(h,c,M)=w_s S(h,c,M)+w_t T(h)+w_r \operatorname{Rep}(h)+w_b B(h)+w_n N(c)-w_d d(h,c) \tag{F-11}
\]

ここで:

- \(S(h,c,M)\): mission 適合性 / locality suitability。[file:24]
- \(T(h)\): trust。
- \(\operatorname{Rep}(h)\): final reputation。
- \(B(h)\): benevolence score。
- \(N(c)\): child need。
- \(d(h,c)\): local village 距離。[file:24]

**意味**: 同程度に有能な adult が複数いるなら、より協力的で評判の良い adult を helper に選ぶ。

### 4.2 softmax helper selection

proposal 候補集合上での重み付けは次でよい。

\[
\pi(h \mid c, M)=
\frac{\exp(\tau_Q Q(h,c,M))}{\sum_{g\in N_t(c)}\exp(\tau_Q Q(g,c,M))} \tag{F-12}
\]

- \(\tau_Q\): 選好の鋭さ。
- 高すぎると helper 固定化、低すぎると benevolence bias が薄まるため calibration candidate とする。[file:24]

### 4.3 bounded remote exploration

v2.3-e の bounded remote exploration を保持しつつ、remote helper 探索率 \(\varepsilon_{remote}\) は benevolence-aware でもよい。例えば local adults の benevolence が十分高い場合は remote exploration を下げ、local shortage 時にのみ上げる。

\[
\varepsilon_{remote}(c)=\operatorname{clip}_{[0,\varepsilon_{max}]}
\left(
\varepsilon_0 + a_1 \cdot \operatorname{need}(c) - a_2 \cdot \overline{B}_{local}(c)
\right) \tag{F-13}
\]

これにより「近くに優しい大人がいるなら、まず近所で助け合う」という世界観を operational に実現できる。

---

## 5. child growth と成熟の組み込み

v2.3-e は child growth と reciprocity / reputation 連携を言及しているため、v2.3-f では具体式を入れる。[file:24]

### 5.1 growth increment

child workflow \(c\) の成長量を次で定義する。

\[
\Delta G_c = \mu_1 \cdot \operatorname{MissionSuccess}_c
+ \mu_2 \sum_h \operatorname{HelpSuccess}(h \to c)
+ \mu_3 \cdot \overline{B}_{helpers(c)}
- \mu_4 \cdot \operatorname{FailureBurden}_c \tag{F-14}
\]

これを `experience_count` や maturation score に反映してよい。

### 5.2 maturation probability

child から adult への成熟判断が存在するなら、benevolence-rich village で成長しやすくするため、次を推奨する。

\[
P_{mature}(c)=\sigma\left(
\nu_0 + \nu_1 E_c^{norm} + \nu_2 T_c + \nu_3 \operatorname{Rep}_c + \nu_4 \overline{B}_{helpers(c)}
\right) \tag{F-15}
\]

**意図**: 優しい大人に囲まれた child は成熟しやすい。Darvium の世界観を、child support village の生態に直結させる。

---

## 6. 実装で追加・更新すべきデータモデル

以下は RFC にそのまま追加可能なレベルの推奨である。

### 6.1 ReputationProfile 拡張

既存の `ReputationProfile` は残しつつ、再計算根拠と観測量を持たせる。[file:24]

```rust
struct ReputationProfile {
    direct_score:       f32,
    indirect_score:     f32,
    experience_score:   f32,
    inherited_score:    f32,
    final_score:        f32,
    alpha_positive:     u32,
    beta_negative:      u32,
    last_recomputed_at: SystemTime,
    direct_help_count:  u32,
    direct_success_count: u32,
    direct_reject_count: u32,
    harm_event_count:   u32,
    accepted_offer_rate: f32,
    help_success_rate:   f32,
    village_centrality:  f32,
    benevolence_score:   f32,
}
```

これらを永続フィールドに追加しない場合でも、recompute 時に導出可能な event source が存在しなければならない。

### 6.2 Reciprocity event log

互恵性再計算のため、Training Plane または runtime metadata に help interaction log を導入することを推奨する。

```rust
struct ReciprocityEvent {
    event_id: String,
    mission_id: String,
    source_graph_id: WorkflowGraphId,
    target_graph_id: WorkflowGraphId,
    event_kind: ReciprocityEventKind,
    weight: f32,
    created_at: SystemTime,
    virtual_clock: u64,
    trace_ref: Option<String>,
}

enum ReciprocityEventKind {
    HelpOffered,
    HelpAccepted,
    HelpRejected,
    HelpExecuted,
    HelpSucceeded,
    HelpAbandoned,
    HarmfulMismatch,
    ReturnedFavor,
}
```

### 6.3 Lifecycle calibration parameter object

```rust
struct ReciprocityLifecyclePolicy {
    theta_dir: f32,
    theta_ind: f32,
    theta_exp: f32,
    theta_inherit: f32,
    lambda_gc_base: f32,
    gamma_lifecycle: f32,
    gamma_benevolence: f32,
    gamma_child_protect: f32,
    rho_direct_decay: f32,
    tau_helper_softmax: f32,
    epsilon_remote_base: f32,
    epsilon_remote_max: f32,
    adult_experience_threshold: u32,
    adult_trust_threshold: f32,
    adult_reputation_threshold: f32,
}
```

これらは `EnvironmentPolicy` 参照下に置いてもよい。重要なのは、**versioned policy object として記録されること**である。[file:24]

---

## 7. 実装フェーズで回すべき較正ループ

v2.3-e の calibration candidate discipline、Training Plane、deterministic replay、property-based test を踏まえると、較正ループは単なる hand tuning ではなく、**観測→ replay → perturbation → parameter update → regression gate** の閉ループとして定義すべきである。[file:24]

## 7.1 較正の目的関数

優しい世界を operational にするため、最終目的は単一指標ではない。multi-objective calibration を採用する。

### 主目的

- 協力的・評判良好な workflow の `P_survive` を上げる。[file:24]
- child support success rate を上げる。[file:24]
- village churn を抑える。[file:24]
- false-new rate を悪化させない。[file:24]
- review-load を暴騰させない。[file:24]

### 推奨 objective

\[
\mathcal{J}(\theta) =
\lambda_1 \cdot \operatorname{AUC}_{benevolent>nonbenevolent}
+ \lambda_2 \cdot \operatorname{HelpSuccessRate}
- \lambda_3 \cdot \operatorname{VillageChurnP95}
- \lambda_4 \cdot \operatorname{FalseNewRate}
- \lambda_5 \cdot \operatorname{ReviewLoad}
- \lambda_6 \cdot \operatorname{InstabilityPenalty} \tag{F-16}
\]

ここで `AUC_{benevolent>nonbenevolent}` は「善良な workflow が非善良 workflow より survival ranking 上位に来る確率」を表す ranking 指標である。

## 7.2 Calibration phases

### Phase 0: pure function validation

まず純粋関数層だけで数式 family を固定する。

実装対象:

- `compute_direct_reciprocity(events, now) -> f32`
- `compute_indirect_reciprocity(graph_metrics) -> f32`
- `recompute_reputation(profile_inputs) -> ReputationProfile`
- `compute_gc_hazard(memo, policy, now, clock) -> f32`
- `compute_survival_probability(hazard, delta_t) -> f32`
- `compute_helper_score(helper, child, mission, policy) -> f32`

この段階では外部依存を一切持ち込まず、Fake-first で unit test を書く。これは RFC の PortTrait / FakeImpl / deterministic replay discipline に一致する。[file:24]

### Phase 1: deterministic replay calibration

既存の TrainingRunLog、SearchTrace、HelpOffer / HelpExecution / HelpSuccess を元に replay dataset を構成し、同一履歴で同一 score / hazard / helper ranking が出ることを保証する。[file:24]

手順:

1. 過去ログから reciprocity event stream を抽出。
2. policy version を固定。
3. recompute を replay。
4. `ReputationProfile.final_score`、`BenevolenceScore`、`GC hazard`、helper ranking をスナップショット比較。

### Phase 2: small perturbation calibration

v2.3 は ranking stability と oscillation risk の replay / property-based test を強調しているため、benevolence integration も small perturbation に耐えなければならない。[file:24]

摂動例:

- help success 1 件追加。
- trust を 0.01 微増減。
- locality distance を微小変更。
- accepted offer を 1 件 rejected に置換。
- 1 helper の reputation を微調整。

観測:

- helper ranking flip rate。
- village churn。
- GC hazard drift。
- survival probability drift。

RFC には「小摂動で unbounded oscillation を起こしてはならない」と明記する。

### Phase 3: synthetic ecosystem simulation

Training Plane の safe sandbox scope で synthetic population を走らせ、優しい世界が emergent に成立するかを検証する。[file:24]

必要な simulator:

- child / adult population generator
- mission stream generator
- locality position updater
- help interaction simulator
- trust / reputation recompute loop
- lifecycle / gc loop

この simulator は production path を汚染せず、Training Plane または fake execution path に限定する。[file:24]

### Phase 4: human-reviewed calibration

最終的な係数更新は human-reviewed でなければならない。RFC の human-centered training / review queue 原則に従い、auto-update を production へ即時反映してはならない。[file:24]

- 候補係数セットを生成。
- replay / simulation で評価。
- 差分レポートを human review queue に送る。
- approve 後に `policy_version` を更新。

---

## 8. パラメータ調整の具体的手順

### 8.1 初期値の置き方

初期値は次の思想で置く。

- `theta_dir` と `theta_ind` は 0 より十分大きくする。
- ただし経験値だけで古参固定化しないよう `theta_exp` は中程度に抑える。
- `gamma_benevolence` は明確に正とし、benevolence が GC hazard を下げる方向を確実にする。
- `tau_helper_softmax` は中程度にし、helper の固定化を避ける。
- `rho_direct_decay` は緩やかにし、過去の善行がすぐ消えないようにする。

### 8.2 調整ルール

RFC 改訂指示書としては、以下のような if-then calibration guide を本文または informative annex に入れるとよい。

- `HelpSuccessRate` が低い → `w_b`, `theta_dir`, `theta_ind` をやや増加し、benevolent helper を選びやすくする。
- `VillageChurnP95` が高い → `tau_helper_softmax` を下げる、または locality smoothing を強める。
- 善良 workflow の survival 優位が弱い → `gamma_benevolence` を増やす。
- 古参固定化が起こる → `theta_exp` を下げ、`kappa_E` を小さくする。
- child が育たない → `mu_2`, `mu_3`, `nu_4`, `gamma_child_protect` を増やす。
- harmful helper が残る → `alpha_d`, `beta_5` を増やし、harm penalty を強める。
- review-load が急増する → village-help proposal のしきい値を上げる、bounded remote exploration を抑える。[file:24]

### 8.3 silent drift 防止

v2.3-e は threshold drift の silent 化を避けるべき文脈を複数持つため、v2.3-f では reciprocity/lifecycle policy も同様に扱う。[file:24]

必須事項:

- policy object に version を付ける。
- SearchTrace / TrainingRunLog / RepairLog 相当の audit object に `policy_version` を残す。[file:24]
- production での係数変更は audit log 必須。
- rollout は canary environment policy から始める。

---

## 9. 監視すべき operational metrics

v2.3-e は reuse quality、false-new rate、repair rate、review-load indicators を前景化している。[file:24] v2.3-f では以下を追加する。

### 9.1 reciprocity / benevolence metrics

- `benevolence_score_p50/p95`
- `direct_reciprocity_p50/p95`
- `indirect_reciprocity_p50/p95`
- `reputation_final_p50/p95`
- `benevolent_survival_advantage`: benevolence 上位群と下位群の survival ratio 差
- `harmful_gc_rate`: harmful score 上位群がどれだけ早く GC されるか

### 9.2 village metrics

- `helper_accept_rate`
- `help_success_rate`
- `help_abandon_rate`
- `village_churn_p50/p95`
- `helper_jsd_p50/p95`
- `remote_exploration_rate`
- `child_maturation_time_mean/p95`
- `child_survival_rate`

### 9.3 regression guard metrics

- `false_new_rate`
- `compose_fallback_frequency`
- `review_queue_depth`
- `review_latency`
- `ranking_flip_rate_under_small_patch`
- `gc_hazard_drift_under_small_patch`

これらは calibration gate に使う。[file:24]

---

## 10. RFC に追加すべきテスト規律

### 10.1 単調性テスト

benevolence integration の最重要 invariant は単調性である。RFC に以下を MUST/SHOULD レベルで書くこと。

- 他条件一定で `direct_score` が増加したら `survival_probability` は減少してはならない。
- 他条件一定で `indirect_score` が増加したら `GC hazard` は増加してはならない。
- 同能力の helper 間で benevolence が高い helper は proposal ranking で不利になってはならない。

### 10.2 replay test

- 同一 event stream、同一 policy version、同一 VirtualClock なら `ReputationProfile` と `GC hazard` の再計算結果は一致すること。[file:24]

### 10.3 perturbation test

- 1 件の help success 追加で village 全体が崩壊的に並び替わらないこと。
- 1 helper の tiny trust change で helper set が全入れ替えしないこと。[file:24]

### 10.4 property-based test

生成対象:

- workflow population size
- child/adult ratio
- distance matrix
- help event stream
- harm/reject noise
- policy coefficients

検証性質:

- benevolence monotonicity
- hazard non-negativity
- probability boundedness
- no negative reputation
- no silent overflow / NaN
- child in grace period is not GC’d regardless of temporary low reputation。[file:24]

---

## 11. 推奨する RFC 本文の差し込み位置

v2.3-f では以下の差し込みが自然である。

### A. §15 Lifecycle / Natural Selection / GC

ここに以下を追加する。

- reciprocity-aware survival principle
- benevolence-aware LifecycleScore or GC hazard
- survival probability formula
- monotonicity constraints
- child protection interaction

### B. §41B Child Support Villages and HELP Consensus Extension

ここに以下を追加する。

- helper weighting への benevolence 項
- direct / indirect reciprocity と help success の接続
- village calibration candidates の追加
- child growth / maturation equations

### C. 付録 / Calibration Candidates

v2.3-e には calibration candidate culture があるため、v2.3-f 用に以下の新規 candidate 群を追加する。[file:24]

- `RECIPROCITY_ALPHA_HELP`
- `RECIPROCITY_ALPHA_SUCCESS`
- `RECIPROCITY_ALPHA_REJECT`
- `RECIPROCITY_ALPHA_HARM`
- `RECIPROCITY_DIRECT_DECAY_RHO`
- `REPUTATION_WEIGHT_DIRECT`
- `REPUTATION_WEIGHT_INDIRECT`
- `LIFECYCLE_WEIGHT_BENEVOLENCE`
- `GC_HAZARD_GAMMA_BENEVOLENCE`
- `GC_HAZARD_GAMMA_CHILD_PROTECT`
- `HELP_WEIGHT_BENEVOLENCE`
- `HELP_SOFTMAX_TAU`
- `REMOTE_EXPLORATION_BASE`
- `REMOTE_EXPLORATION_MAX`
- `CHILD_GROWTH_WEIGHT_HELP_SUCCESS`
- `CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS`

### D. マイルストーン

M0〜M4 あるいは v2.3-e の fake-first implementation plan に次を追加する。

- M0.x: reciprocity pure function + unit tests
- M1.x: replayable reputation/hazard recompute
- M2.x: perturbation suite + ranking stability gate
- M3.x: synthetic village simulator
- M4.x: human-reviewed calibration rollout

---

## 12. 実装タスクへの落とし込み

改訂担当者ではなく実装担当に渡す粒度として、以下のチケット分解を推奨する。

### T1. reciprocity event ingestion

- HelpOffer / HelpExecution / HelpSuccess / rejection / abandonment を `ReciprocityEvent` に落とす。
- SearchTrace / TrainingRunLog と join 可能にする。[file:24]

### T2. reputation recompute engine

- direct / indirect reciprocity を再計算。
- `ReputationProfile.final_score` を policy version 付きで更新。

### T3. lifecycle hazard engine

- existing lifecycle inputs + benevolence inputs から GC hazard を計算。
- dry-run / explain mode を用意し、どの要素が survival に効いたか説明可能にする。

### T4. help ranking integration

- helper proposal score に benevolence を追加。
- softmax / top-k / fallback を deterministic replay 可能に実装。

### T5. child growth integration

- help success と helper benevolence を growth increment に反映。
- maturation observation を Training Plane metrics に追加。

### T6. calibration runner

- replay dataset を流し、candidate policy set を比較評価する CLI または batch runner を用意。
- CSV / JSON report を出す。

### T7. regression gate

- false-new rate、review-load、village churn が悪化した candidate policy を reject する。

---

## 13. 文言上の注意

RFC 改訂では次の書き方を徹底すること。

- 「優しい workflow を優遇する」は抽象表現で終わらせず、**survival probability / GC hazard / helper ranking / maturation** のどこにどう効くかまで書く。
- 「推奨式」と「規範的単調性制約」を分けて書く。
- 実装係数は calibration candidate として扱い、single deployment 内で silent drift させない。[file:24]
- replayability と auditability を必ずセットで書く。[file:24]
- Training / Production separation を破らないことを明記する。[file:24]

---

## 14. 最終的に RFC で達成すべき性質

v2.3-f の改訂後、Darvium は少なくとも次の性質を満たすべきである。

1. 他者を助け、help success を多く生み、harm が少ない workflow は、final reputation が高くなる。[file:24]
2. final reputation と benevolence は GC hazard を下げ、生存確率を上げる。
3. local village で helper を選ぶとき、同程度に能力があるなら、より優しく評判の良い helper が選ばれる。
4. child は benevolent village の中でより育ちやすく、Grace Period と support protection により消えにくい。[file:24]
5. これらの性質は deterministic replay と perturbation test と property-based test で継続的に検証される。[file:24]
6. 係数変更は calibration loop と human review を経て監査可能に導入される。[file:24]

この 6 点が満たされてはじめて、「Darvium の宇宙は優しい世界である」が仕様文として成立する。
