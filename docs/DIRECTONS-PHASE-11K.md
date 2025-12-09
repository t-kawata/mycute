# Phase-11K: Integration Test & Cleanup Plan

## 1. 概要 (Overview)
Phase-11 で実装された全ての機能 (`cubes` API) が連携して正しく動作することを検証する結合テスト（シナリオテスト）の計画です。
テスト完了後、作成されたテストデータが完全に消去（クリーンアップ）される手順も含みます。

## 2. テストシナリオ (Test Scenario)

### Sequential Flow
以下の手順を順番に実行し、全て成功することを確認します。

1.  **Preparation**:
    *   テスト用ユーザー (`TestUser`) を作成・ログイン (JWT取得)。
2.  **Phase 11A: Create**
    *   `POST /v1/cubes/create`: 新規 Cube 作成。
    *   Verify: DBにレコード作成、初期権限 (Unlimited) 確認。
3.  **Phase 11E: Absorb (MemoryGroup テスト)**
    *   `PUT /v1/cubes/absorb`: メモリグループ `legal_expert` にデータ投入。
    *   `PUT /v1/cubes/absorb`: メモリグループ `medical_expert` に別データ投入。
    *   Verify: TokenUsage が加算されていること。
    *   **設計変更**: `memory_group` パラメータが必須。異なるグループに分離して知識を格納。
4.  **Phase 11G: Search (MemoryGroup 分離テスト)**
    *   `GET /v1/cubes/search?memory_group=legal_expert&q=...`: 法律知識のみ検索。
    *   `GET /v1/cubes/search?memory_group=medical_expert&q=...`: 医療知識のみ検索。
    *   Verify: 各グループから適切な回答が返されること（混在しないこと）。
    *   **設計変更**: `memory_group` パラメータが必須。
5.  **Phase 11C: Export**
    *   `GET /v1/cubes/export`: Export実行。
    *   Verify: `Export` レコード作成、NewUUID 発行、Lineage更新。
    *   **注意**: Export は全 MemoryGroup を含む。
6.  **Phase 11J: GenKey**
    *   `POST /v1/cubes/genkey`: Exportされた Cube (NewUUID) に対する鍵発行。
    *   Limit設定: `AbsorbLimit=1`, `Expire=1h`.
    *   Verify: 鍵生成成功。
7.  **Phase 11B: Import**
    *   `POST /v1/cubes/import`: 上記鍵とExportファイルでインポート (別名で)。
    *   Verify: `BurnedKey` 追加、`Permissions` が設定通り (`AbsorbLimit=1`) であること。
    *   Verify: Import後、両方の MemoryGroup (`legal_expert`, `medical_expert`) がアクセス可能であること。
8.  **Phase 11E: Absorb (Limited)**
    *   `PUT /v1/cubes/absorb`: 1回目実行 -> Success.
    *   Verify: Limit が `1 -> -1` に更新されること。
    *   `PUT /v1/cubes/absorb`: 2回目実行 -> **Failure (403 Forbidden)**.
9.  **Phase 11H: Delete**
    *   `DELETE /v1/cubes/delete`: ImportしたCubeを削除。
    *   Verify: DBレコード論理削除(または物理削除)、ファイル消失。

## 3. クリーンアップ手順 (Cleanup Procedure)

テスト終了後、以下の手順でゴミデータを完全に削除します。

### 自動化スクリプト (SQL & Shell)
`scripts/clean_test_data.sh` (仮) 等を作成して実行します。

1.  **DB Cleanup**:
    *   `TestUser` の ID に紐づく全ての `cubes`, `exports`, `burned_keys`, `cube_model_stats`, `cube_contributors`, `cube_lineages` を削除。
    *   `TestUser` 自体を削除。
    ```sql
    DELETE FROM cube_model_stats WHERE apx_id = ? AND vdr_id = ?;
    -- ... (cascade manually if needed)
    ```
2.  **File System Cleanup**:
    *   `DB_DIR_PATH` 下の `apx-vdr-testusr` ディレクトリを `rm -rf`。

## 4. 実行要領
*   `make test-integration` のようなターゲットを作成し、Go の `testing` パッケージまたは外部スクリプトから実行する。
*   テスト環境 (`mode=test`) で実行し、本番データに影響を与えないこと。
