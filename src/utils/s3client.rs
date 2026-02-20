use crate::utils::time;
use anyhow::{anyhow, Result};
use regex::Regex;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use std::path::{Path, PathBuf};
use tokio::fs;

/// S3とローカルストレージを抽象化するクライアント構造体
pub struct S3Client {
    // Bucket::new は Box<Bucket> を返すためそのまま保持
    bucket_obj: Option<Box<Bucket>>,
    // Go版 IsValidS3Settings() で使用するフィールド群
    access_key: String,
    secret_key: String,
    bucket_name: String,
    region_str: String,

    local_dir: PathBuf,
    down_dir: PathBuf,
    use_local: bool,
}

impl S3Client {
    /// 新しいS3Clientインスタンスを作成します
    pub async fn new(
        access_key: &str,
        secret_key: &str,
        region: &str,
        bucket: &str,
        local_dir: &str,
        down_dir: &str,
        use_local: bool,
    ) -> Result<Self> {
        if [access_key, secret_key, region, bucket, local_dir]
            .iter()
            .any(|s| s.is_empty())
        {
            return Err(anyhow!("Invalid arguments."));
        }

        let bucket_obj = if !use_local {
            let region_enum: Region = region
                .parse()
                .map_err(|_| anyhow!("Invalid region: {}", region))?;
            let creds = Credentials::new(Some(access_key), Some(secret_key), None, None, None)?;
            // Bucket::new は Box<Bucket> を返す
            Some(Bucket::new(bucket, region_enum, creds)?)
        } else {
            None
        };

        Ok(Self {
            bucket_obj,
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
            bucket_name: bucket.to_string(),
            region_str: region.to_string(),
            local_dir: PathBuf::from(local_dir),
            down_dir: PathBuf::from(down_dir),
            use_local,
        })
    }

    /// ファイルをアップロードし、保存先のキーを返します
    pub async fn up(&self, file_path: &str) -> Result<String> {
        let now = time::now();
        let dir_key = now.format("%Y/%m/%d_%H-%M").to_string();
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid file path"))?;
        let full_key = format!("{}/{}", dir_key, file_name);

        if self.use_local {
            let dest_dir = self.local_dir.join(&dir_key);
            fs::create_dir_all(&dest_dir).await?;
            fs::copy(file_path, dest_dir.join(file_name)).await?;
        } else if let Some(bucket) = &self.bucket_obj {
            if !self.is_valid_s3_settings() {
                return Err(anyhow!("Invalid S3 settings."));
            }
            let mut file = fs::File::open(file_path).await?;
            bucket
                .put_object_stream(&mut file, &full_key)
                .await
                .map_err(|e| anyhow!("S3 upload failed: {}", e))?;
        } else {
            return Err(anyhow!("S3 Bucket not initialized"));
        }
        Ok(full_key)
    }

    /// ファイルをDLし、ローカルのキャッシュパスを返します
    pub async fn down(&self, path_from_up: &str) -> Result<PathBuf> {
        let clean_key = path_from_up.trim_start_matches('/');
        let cache_path = self.down_dir.join(clean_key);

        // キャッシュ済みなら即座に返却
        match fs::metadata(&cache_path).await {
            Ok(_) => return Ok(cache_path),
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => (),
        }

        // local_dir（アップロード済み実体）を確認
        let local_source = self.local_dir.join(clean_key);
        match fs::metadata(&local_source).await {
            Ok(_) => {
                fs::create_dir_all(cache_path.parent().unwrap()).await?;
                fs::copy(&local_source, &cache_path).await?;
                return Ok(cache_path);
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => (),
        }

        if self.use_local {
            return Err(anyhow!("File not found locally."));
        }

        // S3から取得してキャッシュに保存
        if let Some(bucket) = &self.bucket_obj {
            if !self.is_valid_s3_settings() {
                return Err(anyhow!("Invalid S3 settings."));
            }
            let response_data = bucket
                .get_object(clean_key)
                .await
                .map_err(|e| anyhow!("Failed to get object from S3: {}", e))?;
            let data = response_data.bytes();

            fs::create_dir_all(cache_path.parent().unwrap()).await?;
            fs::write(&cache_path, data).await?;
        } else {
            return Err(anyhow!("S3 Bucket not initialized"));
        }

        Ok(cache_path)
    }

    /// ファイルを削除し、空になった親ディレクトリを掃除します
    pub async fn del(&self, path_from_up: &str) -> Result<()> {
        let clean_key = path_from_up.trim_start_matches('/');
        let mut local_err: Option<anyhow::Error> = None;
        let mut s3_err = None;

        // ダウンロードキャッシュの削除
        let cache_path = self.down_dir.join(clean_key);
        if fs::metadata(&cache_path).await.is_ok() {
            if let Err(e) = fs::remove_file(&cache_path).await {
                return Err(anyhow!("Failed to delete local-down-cache-file: {}", e));
            }
            let _ = self
                .tidy_up_dirs(&self.down_dir, cache_path.parent().unwrap())
                .await;
        }

        // ローカル実体の削除
        let local_path = self.local_dir.join(clean_key);
        if fs::metadata(&local_path).await.is_ok() {
            if let Err(e) = fs::remove_file(&local_path).await {
                local_err = Some(e.into());
            } else {
                let _ = self
                    .tidy_up_dirs(&self.local_dir, local_path.parent().unwrap())
                    .await;
            }
        }

        // S3から削除
        if !self.use_local {
            if !self.is_valid_s3_settings() {
                s3_err = Some(anyhow!("Invalid S3 settings."));
            } else if let Some(bucket) = &self.bucket_obj {
                if let Err(e) = bucket.delete_object(clean_key).await {
                    s3_err = Some(anyhow!("Failed to delete S3 object: {}", e));
                }
            } else {
                s3_err = Some(anyhow!("S3 Bucket not initialized"));
            }
        }

        match (local_err, s3_err) {
            (Some(le), Some(se)) => {
                Err(anyhow!("Failed local and S3: loc: {:?}, s3: {:?}", le, se))
            }
            (Some(le), None) => Err(anyhow!("Failed locally: {:?}", le)),
            (None, Some(se)) => Err(anyhow!("Failed S3: {:?}", se)),
            (None, None) => Ok(()),
        }
    }

    /// 指定の正規表現にマッチするオブジェクトを走査します
    /// Go版と同様に、走査エラーに関わらず finalCallback を必ず呼ぶ
    pub async fn walk<F, Fut, G, GFut>(
        &self,
        re: Regex,
        callback: F,
        interval_ms: u64,
        final_callback: Option<G>,
    ) -> Result<()>
    where
        F: Fn(String, String) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
        G: Fn() -> GFut,
        GFut: std::future::Future<Output = Result<()>>,
    {
        // 走査エラーを一時保持し、finalCallback を必ず呼んでから返す (Go版準拠)
        let mut walk_err: Option<anyhow::Error> = None;

        if self.use_local {
            for entry in walkdir::WalkDir::new(&self.local_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    if re.is_match(&filename) {
                        let rel_path = match entry.path().strip_prefix(&self.local_dir) {
                            Ok(p) => p.to_string_lossy().to_string(),
                            Err(e) => {
                                walk_err = Some(e.into());
                                break;
                            }
                        };
                        if let Err(e) = callback(rel_path, filename).await {
                            walk_err = Some(e);
                            break;
                        }
                        if interval_ms > 0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms))
                                .await;
                        }
                    }
                }
            }
        } else if let Some(bucket) = &self.bucket_obj {
            if !self.is_valid_s3_settings() {
                walk_err = Some(anyhow!("Invalid S3 settings."));
            } else {
                match bucket.list("".to_string(), None).await {
                    Ok(results) => {
                        'outer: for list_result in results {
                            for obj in list_result.contents {
                                let key = obj.key;
                                let filename = Path::new(&key)
                                    .file_name()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                if re.is_match(&filename) {
                                    if let Err(e) = callback(key, filename).await {
                                        walk_err = Some(e);
                                        break 'outer;
                                    }
                                    if interval_ms > 0 {
                                        tokio::time::sleep(tokio::time::Duration::from_millis(
                                            interval_ms,
                                        ))
                                        .await;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        walk_err = Some(anyhow!("Failed to list S3 objects: {}", e));
                    }
                }
            }
        }

        // Go版と同様に、走査エラーに関わらず finalCallback を必ず呼ぶ
        if let Some(cb) = final_callback {
            if let Err(cb_err) = cb().await {
                return Err(cb_err);
            }
        }

        match walk_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// ファイルの存在確認
    pub async fn is_exist(&self, path_from_up: &str) -> bool {
        let clean_key = path_from_up.trim_start_matches('/');
        if fs::metadata(self.local_dir.join(clean_key)).await.is_ok() {
            return true;
        }
        if self.use_local {
            return false;
        }
        if !self.is_valid_s3_settings() {
            return false;
        }

        if let Some(bucket) = &self.bucket_obj {
            bucket.head_object(clean_key).await.is_ok()
        } else {
            false
        }
    }

    /// 再帰的に空のディレクトリを削除する内部関数
    async fn tidy_up_dirs(&self, root: &Path, start: &Path) -> Result<()> {
        let mut current = start;
        while current != root && current.to_str() != Some("/") {
            let mut entries = fs::read_dir(current).await?;
            if entries.next_entry().await?.is_none() {
                fs::remove_dir(current).await?;
                current = current.parent().ok_or_else(|| anyhow!("Root reached"))?;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// S3 への接続と権限を確認します。
    /// ローカルモードの場合はディレクトリが存在しなければ作成します。
    pub async fn validate_connection(&self) -> Result<()> {
        if self.use_local {
            Ok(())
        } else if let Some(bucket) = &self.bucket_obj {
            // list を実行して接続を検証
            bucket
                .list("".to_string(), None)
                .await
                .map(|_| ())
                .map_err(|e| anyhow!("Failed to validate S3 connection: {}", e))
        } else {
            Err(anyhow!("S3 bucket not initialized for validation"))
        }
    }

    /// Go版 IsValidS3Settings() に相当するガードメソッド。
    /// 設定値が "empty" プレースホルダのままの場合 false を返す。
    fn is_valid_s3_settings(&self) -> bool {
        let empty = "empty";
        !(self.access_key == empty
            || self.secret_key == empty
            || self.bucket_name == empty
            || self.region_str == empty)
    }
}
