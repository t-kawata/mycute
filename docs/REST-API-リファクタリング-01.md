# 1. 整理内容の概要

## 1-1. ノードの区分
- OWNER
- CA
- NODE（一般ユーザーまたは開発者）

## 1-2. 時系列を以下にまとめる
1. 全てのNODEが初回起動時に自動でキーペアを作成し、
    1-1. 自分のノードの POST /v1/node/identities/entry を叩く（リクエストボディに `ca_base_url` を含める）
    1-2. POST /v1/node/identities/entry の処理内にて `ca_base_url` 先の POST /v1/ca/identities/entry（元 POST /v1/identities）を叩く
    1-3. POST /v1/ca/identities/entry からのレスポンスを POST /v1/node/identities/entry でレスポンス
2. NODEは、アプリを探すために
    2-1. 自分のノードの POST /v1/node/apps/discover を叩く
    2-2. POST /v1/node/apps/discover の処理内にて POST /v1/ca/apps/discover（元 POST /v1/apps/discover）を叩く
    2-3. POST /v1/ca/apps/discover からのレスポンスを POST /v1/node/apps/discover でレスポンス
3. NODEが気に入ったアプリを見つけたら、
    3-1. mycuteファイルを手に入れて（この部分は後日実装）、
    3-2. 自分のノードの POST /v1/node/apps/install/file（元 POST /v1/apps/install/file）でインストール
4. NODEは、
    4-1. 自分のノード内の全てのアプリを GET /v1/pub/apps/list（元 GET /v1/apps/list）にてパブリックに公開しているので、
    4-2. 誰でも誰がどのアプリを持っているかを知ることができる。
    4-3. もちろんCAによる情報収集にも使用される。
5. NODEは現在使用しているアプリが気に入ったら、
    5-1. 自分のノードの POST /v1/node/apps/vote を叩く
    5-2. POST /v1/node/apps/vote の処理内にて POST /v1/ca/apps/vote（元 POST /v1/apps/vote）を叩く
    5-3. POST /v1/ca/apps/vote からのレスポンスを POST /v1/node/apps/vote でレスポンス
6. NODEはアプリを使用するだけでなく、開発もしてみたくなり、アプリを開発した後、
    6-1. POST /v1/node/apps/build（元 POST /v1/apps/build）でビルドしてmycuteファイルを得る
    6-2. この時点では自己署名だけの L1 アプリであり信用度は低いが、友人たちに配布できる（起動時にアラートが出る）
    6-3. L1 アプリであっても、自分のノードの POST /v1/node/apps/advertise を叩いて広告でき、
    6-4. POST /v1/node/apps/advertise 処理内にて POST /v1/ca/apps/advertise（元 POST /v1/apps/advertise）が叩かれ
    6-5. POST /v1/ca/apps/advertise からのレスポンスを POST /v1/node/apps/advertise でレスポンス
7. 自己署名 L1 アプリが少し人気になってきたので、アプリの信用力を上げるために
    7-1. 自分のノードの POST /v1/node/identities/apply を叩いて本人確認申請を開始
    7-2. POST /v1/node/identities/apply 処理内にて POST /v1/ca/identities/apply（元 POST /v1/identities/apply）が叩かれ
    7-3. POST /v1/ca/identities/apply からのレスポンスを POST /v1/node/identities/apply でレスポンス
    7-4. 申請先のCAの独自のKYC手続きにしたがって本人確認を完了させる
    7-5. 申請先のCAは、KYCが完了した通知をemailなどで行う
    7-6. 本人確認完了通知を受けたNODEは、自分のノードの POST /v1/node/identities/sync を叩く
    7-7. POST /v1/node/identities/sync の処理内にて POST /v1/ca/identities/sync（元 GET /v1/identities/retrieve/{pubkey}) が叩かれ、
    7-8. POST /v1/ca/identities/sync からのレスポンスを POST /v1/node/identities/sync で受け取り、
    7-9. POST /v1/node/identities/sync の処理内にて、NODEのDBに情報を格納することで、
    7-10. このNODEは、L1 NODEから L2 以上に昇格することができる。
    7-11. この時、CAがOWNERからの任命書を持つCAだった場合には L3 昇格で、任命署を持たない野良CAだった場合には L2 昇格となる。
8. CAによる本人確認によって L2 以上に昇格したNODEは、
    8-1. 上記 6-1 ~ 6-5 までと同一の手続きによって再びアプリを公開すると
    8-2. 新たに行なったビルドの時点で、最新の信用情報がmycuteファイルに自動で書き込まれるため
    8-3. アプリ自体の信用力が上がった状態で広告することが可能であり、
    8-4. 上記7の手続きによって L3 に昇格していた場合は、アプリ起動時のアラートは消える
    8-5. L3 以上の場合に限り、上記5の投票を受けたアプリは投票を受けた数に対する特別な計算により L4 以上に無限に成長する
    8-6. この時、「投票」と「L（信用度）の再計算」には、CA内での計算サイクルに応じてラグがあるため、
    8-7. NODEは、自分のノードの POST /v1/node/apps/recalc を叩くことで、CAに対して自分のアプリの信用度再計算を依頼でき、
    8-8. POST /v1/node/apps/recalc の処理内にて POST /v1/ca/apps/recalc が叩かれ、
    8-9. POST /v1/ca/apps/recalc からのレスポンスを POST /v1/node/apps/recalc でレスポンスする

## 1-3. CAによるアイデンティティ管理用エンドポイント
    - POST /v1/identities/search -> POST /v1/ca/identities/search に変更
    - GET /v1/identities/{identity_id} -> GET /v1/ca/identities/{identity_id} に変更
    - PUT /v1/identities/{identity_id}/verify -> PUT /v1/ca/identities/{identity_id}/verify に変更
    - DELETE /v1/identities/{identity_id} -> DELETE /v1/ca/identities/{identity_id} に変更

## 1-4. 完全パブリックなエンドポイント
    - GET /v1/identities/pubkey -> GET /v1/pub/identities/pubkey に変更
    - GET /v1/apps/list -> GET /v1/pub/apps/list に変更
    - GET /v1/ca/status -> GET /v1/pub/ca/status に変更

# 「1. 全てのNODEが初回起動時に自動でキーペアを作成し、」の実装計画

- [x] **キーペアの自動作成**
    - **所在**: `src/mode/rt/rtbl/identities_bl.rs` -> `fn ensure_node_identity`
    - **呼出**: `src/mode/rt/main_of_rt.rs` のサーバー起動時に実行。
    - **内容**: `settings.json` にキーが存在しない場合、Ed448 キーペアを新規生成し、暗号化して保存している。

- [x] **CA側エンドポイント整備: `POST /v1/ca/identities/entry`**
    - **現状**: `src/mode/rt/rthandler/identities_handler.rs` -> `fn create_identity` が機能的に相当 (`POST /v1/identities`)。
    - **作業**:
        - 本質的なロジックは流用可能。
        - エンドポイントパスを `/v1/ca/identities/entry` に変更（`src/mode/rt/req_map.rs`）。
        - ハンドラ関数名やリクエスト構造体名を `entry_identity` / `EntryIdentityReq` 等にリネームして意味を明確化。

- [x] **Node側エンドポイント作成: `POST /v1/node/identities/entry`**
    - **現状**: 未実装。
    - **作業**:
        - `src/mode/rt/rthandler/node_identities_handler.rs` を新規作成。
        - ハンドラ `fn entry_identity` を実装。
        - **処理内容**:
            1. リクエストボディから目的地となる `ca_base_url` を受け取る。
            2. 自身の公開鍵（`identities_bl::get_pubkey`）を取得。
            3. `ca_base_url` 先の `POST /v1/ca/identities/entry` を HTTP クライアントで叩く。
            4. CA からのレスポンス（登録完了通知）を受け取り、クライアントに返す。


# 「2. NODEは、アプリを探すために」の実装計画

- [x] **CA側エンドポイント整備: `POST /v1/ca/apps/discover`**
    - **所在**: `src/mode/rt/rthandler/ca_apps_handler.rs` -> `fn discover_app_ca`
    - **作業**:
        - `apps_handler.rs` を `ca_apps_handler.rs` にリネームし、他のハンドラ（Advertise, Vote等）も CA 用として整理。
        - エンドポイントパスを `/v1/ca/apps/discover` に変更。
        - **CA側スケルトンロジック**: `ca_apps_bl::discover_app` を実装。現時点では P2P 探索は行わず、固定のノード URL リスト（モック）を返却する。
        - レスポンス構造体 `DiscoverAppCaRes` を使用。

- [x] **Node側エンドポイント作成: `POST /v1/node/apps/discover`**
    - **所在**: `src/mode/rt/rthandler/node_apps_handler.rs` -> `fn discover_app_node`
    - **作業**:
        - `node_apps_handler.rs` を新規作成。
        - ハンドラ `fn discover_app_node` を実装。
        - **処理内容**:
            1. リクエストボディ (`DiscoverAppNodeReq`) から `app_id` と `ca_base_url` を受け取る。
            2. `ca_base_url` 先の `POST /v1/ca/apps/discover` を `reqwest` クライアントで呼び出す。
            3. CA からのレスポンス (`DiscoverAppCaRes`) を受け取り、`DiscoverAppNodeRes` としてラップして返却する。

- [x] **物理的なファイル構成の整理 (Node/CA 分離)**
    - **作業**:
        - `rtreq`, `rtres`, `rtbl` の各レイヤーで `apps` 関連を `node_apps_*` と `ca_apps_*` に分離し、`mod.rs` を更新する。
        - 全ての構造体名に `Node` または `Ca` プレフィックスを付与し、意図を明確にする。
        - `req_map.rs` のルーティングを新設されたハンドラに合わせて更新する。

# 「3. NODEが気に入ったアプリを見つけたら、」の実装計画

- [x] **Node側エンドポイント整備: `POST /v1/node/apps/install/file`**
    - **所在**: `src/mode/rt/rthandler/node_apps_handler.rs` -> `fn install_app_file_node`
    - **作業**:
        - CA側から機能を完全に排除し、Node専用の機能として定義する。
        - マルチパート形式 (`multipart/form-data`) で `.mycute` ファイルを受け取る。
        - 署名検証とインストール処理を `node_apps_bl::install_app_file_node` に委譲する。

- [x] **インストールの正確性と安全性の点検・確定**
    - **所在**: `src/mode/rt/rtbl/node_apps_bl.rs` -> `fn install_app_file_node`
    - **完了条件**:
        - **一時ディレクトリの安全性**: `tempfile` を使用し、エラー発生時も含めてクリーンアップが保証されている。
        - **パッケージ検証の厳格化**: `pkg_bl::extract_package` を呼び出し、署名、マニフェスト、ハッシュの整合性が検証されている。
        - **インストールパスの管理**: `~/.mycute/apps/<app_id>` への正確な配置。
        - **DBレコードの正規化**: 既存の `app_id` がある場合は `Update`、ない場合は `Insert` し、`package_hash` や `verified_level` が正しく記録されている。
        - **レスポンス品質**: インストール後のアプリ詳細情報を `AppInfoNodeRes` として正しく返す。

# 「4. NODEは、」の実装計画

- [x] **Public側エンドポイント作成: `GET /v1/pub/apps/list`**
    - **所在**: `src/mode/rt/rthandler/pub_apps_handler.rs` (新規作成) -> `fn list_apps_pub`
    - **作業**:
        - 認証を必要としないパブリックなエンドポイントとして `/v1/pub/apps/list` を提供。
        - 呼び出し側の事前知識（ID等）に依存しないよう、パスパラメータを排除。

- [x] **信頼の検証を完結させるためのレスポンス強化**
    - **作業**:
        - `AppInfoPubItemRes` に、検証に必要な全パーツ（Rawマニフェスト、署名チェーン、CA公開鍵等）を追加する。
        - 取得側がシステムのルート（Anchor）から数学的な検証をバイナリなしで完結できることを保証する。
        - **セキュリティとコミットメント**: 開発者がアプリを将来にわたって改竄しないことを世界に誓約する証拠（ハッシュと署名のセット）として機能させる。

# 「5. NODEは現在使用しているアプリが気に入ったら、」の実装計画

## 5.1. MYCUTE における「投票」の本質と哲学
MYCUTE の投票は、単なる「いいね！」ボタンではありません。それは **「二次投票 (Quadratic Voting) を用いた、熱意（クレジット）の社会的投資」** です。

- **L2 / L3 共通の表現尺度 (1-15 票)**:
    - ノードの身分にかかわらず、一つのアプリに対して投じることができる「票数」は最大 15 票に共通化します。
    - これにより、ユーザーの感情的な尺度（愛着）を不変のデータとして残し、身分の変化（L2 -> L3）に対して頑健なシステムを構築します。
- **動的なスコアリング（計算レイヤーでの重み付け）**:
    - **L3 ノード**: 投じた $n$ 票に対し、$n^2$ の二乗則を適用し（最大 225 ポイント）、強力な社会的インパクトとして評価します。
    - **L2 以下ノード**: 同じ $n$ 票であっても、$n / 15$ としてリニアに減衰させ（最大 1.0 ポイント）、シビル攻撃に対する防波堤とします。
- **昇格時のブースト**: 
    - L2 時代に投じた 15 票は、本人が L3 に昇格した瞬間、メンテナンス不要で自動的に 225 ポイントの威力を持つ「責任ある市民の声」へとアップグレードされます。

## 5.2. 実装時の厳格な注意点 (Accident Prevention)

### A. アイデンティティと権限の動的検証
- **「票数」と「影響力」の分離**: Node 側の API および DB では「1-15 票」という純粋な意図のみを保存・転送し、具体的なスコアリング（二乗するか $1/15$ するか）の判断は、集計を行う CA レイヤーでの動的判定に委ねること。
- **二重投票の排除**: `apx_id` / `vdr_id` にかかわらず、一つの物理ノード（PubKey）から一つのアプリ（AppId）への有効な投票データは常に一件（最新の上書き）に制限すること。

### B. 二乗コスト (Quadratic Cost) の適用
- **熱意の重み付け**: L3が $n$ 票を投じる際、システム内部では必ず $n^2$ の重みとして処理されること。API のパラメータで「票数」を受け取るが、バックエンドでの集計ロジック（CAへの送信時）でこの二乗則が適用されなければならない。
- **負の投票の禁止**: 今回のフェーズでは「正の熱意（賛成）」のみを扱うこと。マイナスの投票は複雑性を招くため排除する。

### C. プロキシとしての Node の責務と署名
- **投票の署名付与**: `node/vote` は `ca/vote` の単なるパススルーではない。Node はユーザーのリクエストを受け取った後、**「自身のノード署名」** を付与して CA に転送しなければならない。これにより、CA は「どこの誰からの投票か」を検証できる。
- **信頼の接続先**: Node は自身が所属・信頼している CA に対してのみ投票を送信すること。ランダムな CA への送信は、票の蒸逸や改竄のリスクとなる。

### D. 状態としての投票 (Persistence)
- **永続的な意思表明**: 投票は一時的なイベントではなく「状態」である。一度投じた票は、ユーザーが取り下げるか変更するまで永続的に有効であり、DB に記録され続ける必要がある。

### E. 用語の厳格な定義 (Terminology: Balance vs Credits)
- **バランス (Balance)**:
  - 「投票できる権利の量（手持ちの弾数）」を指す。
  - CAからNodeに付与され、Nodeが保持し、投票によって消費していくもの。
  - コード上では `initial_balance`, `current_balance` と表現されるべきである。
- **クレジット (Credits)**:
  - 「投票結果として生み出される社会的価値（スコア）」を指す。
  - Quadratic Voting (QV) の計算結果として算出される値（$n^2$ 等）。
  - 消費するものではなく、投票によってアプリや対象に与えられる信用スコアそのものを指す。

## 5.3. 実装計画 (Implementation Plan Checklist)

### 【5.3.0】 エントリー機能の拡張 (予算証書の授与)
- [x] 投票を行うための「資源（予算）」と「身分の証明」を、アイデンティティ・エントリーのタイミングで CA から取得します。

- **CA側 (`POST /v1/ca/identities/entry`)**:
  - [x] **初期予算証書の生成**: `{ node_pubkey, initial_credits: 15, issued_at }` を CA の秘密鍵で署名。
  - [x] **CAトークンの同梱**: オーナーから正式に任命されている場合、任命証（CA Token）をレスポンスに含める。
- **Node側 (`POST /v1/node/identities/entry`)**:
  - [x] **`node_tickets` テーブルへの保存**: 複数の CA からエントリーを受けることで、信頼のポートフォリオ（連合アイデンティティ）を構築。
  - [x] **予算の初期化 (`my_rem`)**: 最初の証書を受け取った際、設定 JSON 内の **`my_rem`** フィールド（暗号化済み）を作成し、残数 15 と自己署名を格納して封印する。

### 【5.3.1】 自分のノードの `POST /v1/node/apps/vote` を叩く (Entry Point)
- [ ] ユーザーは自身のノード（影分身）のコンテキストで、特定のアプリ（App ID）に対して 1〜15 票の熱意を表明します。

- **Handler**: `src/mode/rt/rthandler/node_apps_handler.rs` -> `vote_app_node`
- **Request**:
  - [ ] `app_id` (String: UUID)
  - [ ] `ca_base_url` (String: 投票を中継する CA の URL)
  - [ ] `vote_count` (u32: 1〜15)
- **処理**:
  1. [ ] `vote_count` が 1〜15 の範囲内であることをバリデーション。
  2. [ ] `node_apps_bl::vote_app_node` を呼び出す。

### 【5.3.2】 `POST /v1/node/apps/vote` 内で `POST /v1/ca/apps/vote` を叩く (Proxy & Signing & Budget)
- [ ] Node 側は単なるリレーではなく、自身のノードとしての「意思」を証明し、かつ「資源（投票権）」を管理する責務を持ちます。

- **Business Logic**: `src/mode/rt/rtbl/node_apps_bl.rs` -> `vote_app_node`
- **処理ステップ**:
  1. **投票予算（クレジット）の確認と改竄検証**:
     - [ ] **連合証書の確認**: `node_tickets` テーブルから、保持しているすべての「初期予算証書（Ticket）」を取得。
     - [ ] **ローカル改竄検証**: 設定 JSON の **`my_rem`** フィールドを復号し、`{credits}:{signature}` の整合性を検証。
     - [ ] **コスト計算**: 今回の投票に必要な票数（$n_{new} - n_{old}$）を計算し、残数が十分であるかを確認。
  2. **予算の更新と再署名**:
     - [ ] 投票後の新残数を計算し、自身の秘密鍵で新たな署名を生成。それらを連結・暗号化して設定 JSON の `my_rem` を更新する。
  3. **リクエストの構築**: CA 向けのリクエスト (`VoteAppCaReq`) を作成。
     - [ ] **PubKey の封入**: 公開鍵をリクエストボディに格納。
     - [ ] **証書の束（Bundle）の添付**: 保持しているすべての予算証書（CAトークン含む）を配列としてリクエストに含める。
  4. **ノード署名の付与**: 
     - [ ] `{app_id, vote_count, timestamp, pub_key, tickets}` などのデータセット全体に対して署名を行う。
  5. [ ] **CA への HTTP リクエスト**: `reqwest` を使用し、CA へ `POST`。

### 【5.3.3】 `POST /v1/ca/apps/vote` で票を受け取り、集計・レスポンスする (Scoring & Persistence)
- [ ] 信頼のハブである CA は、受け取った票を「市民権（L3）の有無」に基づいて評価し、スコアリングを確定させます。

- **Handler**: `src/mode/rt/rthandler/ca_apps_handler.rs` -> `vote_app_ca`
- **Business Logic**: `src/mode/rt/rtbl/ca_apps_bl.rs` -> `vote_app_ca`
- **処理ステップ**:
  1. [ ] **署名検証**: 届いたリクエストボディの署名が、添付された PubKey によるものであることを検証。
  2. **証拠の整合性検証 (Federated Audit)**:
     - [ ] 添付された「証書の束（Ticket Bundle）」に含まれる全証書が、各発行元 CA により正当に署名されているかを確認。
     - [ ] **公平な受理**: 証書の中に CA トークン（L3証明）が含まれていなくても、リクエストが数学的に正当である限り CA は投票を拒絶しない。
  3. **投票の受理（永続化のみ）**:
     - [ ] **注記**: ここではインパクト計（二乗計算など）は行わず、「誰が、どのアプリに、何票投じたか」という意志（Intent）を DB に保存することに専念する。
  4. [ ] **返却**: インパクト算出前である暫定的なレスポンスを返却。

### 【5.3.4】 社会的インパクトの確定 (Recalc & Background)
- [ ] 投票の受理と計算（Impactの確定）を時間軸で分離します。

- **オンデマンド更新 (`POST /v1/node/apps/recalc`)**:
  - [ ] Node から CA に `app_id` を送信し、特定のアプリについて「今すぐ社会的スコアを再計算してカタログを更新せよ」と依頼する（プロキシ）。
- **CA 側のバックグラウンド・タスク**:
  - [ ] 設定 JSON 内の `recalc_cycle_seconds` (例: 3600) に従い、常駐ワーカーが全アプリの社会的インパクトをバッチ処理で再計算し、カタログを最新の状態にする。

### 【補足】 データ構造と改竄防止 (Tamper-proof)
- [ ] **新規テーブル `node_tickets` (Edge側)**: 複数の CA からの証書 (`ticket_json`, `ca_token`) を格納。
- [ ] **予算情報の保存先**: 設定 JSON (`settings_mac.json` 等) の Identity ブロック内の **`my_rem`** フィールド。
- [ ] **保存形式**: `"{credits}:{signature}"` という形式で連結し、`my_sec` 等と同様の方式で暗号化した単一の文字列。
- [ ] **運用**: Entry 時に授与された証拠を Root とし、以降の残数推移を秘密鍵で自己署名し、暗号化して封印し続ける。

### 【補足2】 コスト計算と「撤回・更新」のロジック
- [ ] 投票は単純な加算ではなく、常に「最新の状態」を維持し、差分を予算に反映させます。
- [ ] **計算式**: 消費コスト $C = n$ ($n$: 投票数, 0〜15)
- [ ] **差分処理**:
    - [ ] 前回の投票数が $n_{old}$、今回の投票数が $n_{new}$ の場合：
    - [ ] 消費予算 = $n_{new} - n_{old}$
- [ ] **撤回**: $n_{new} = 0$ とすることで、$0 - n_{old}$ が計算され、過去の消費分が予算（15）に全額払い戻しされます。
- [ ] **更新**: 例えば 5 票から 10 票へ変更する場合、$(10 - 5) = 5$ 票が追加で消費されます。


# 「6. NODEはアプリを使用するだけでなく、開発もしてみたくなり、アプリを開発した後、」の実装計画

# 「7. 自己署名 L1 アプリが少し人気になってきたので、アプリの信用力を上げるために」の実装計画

# 「8. CAによる本人確認によって L2 以上に昇格したNODEは、」の実装計画