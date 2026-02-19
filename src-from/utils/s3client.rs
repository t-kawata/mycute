use anyhow::{anyhow, Result};
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use chrono::Local;
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::fs;

/// S3とローカルストレージを抽象化するクライアント構造体
pub struct S3Client {
    client: Client,
    access_key: String,
    secret_key: String,
    bucket: String,
    region: String,
    local_dir: PathBuf,
    down_dir: PathBuf,
    use_local: bool,
}

impl S3Client {
    /// 新しいS3Clientインスタンスを作成します (Go: NewS3Client)
    pub async fn new(
        access_key: &str,
        secret_key: &str,
        region: &str,
        bucket: &str,
        local_dir: &str,
        down_dir: &str,
        use_local: bool,
    ) -> Result<Self> {
        if [access_key, secret_key, region, bucket, local_dir].iter().any(|s| s.is_empty()) {
            return Err(anyhow!("Invalid arguments."));
        }

        let region_obj = aws_config::Region::new(region.to_string());
        let creds = Credentials::new(access_key, secret_key, None, None, "static");

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_obj)
            .credentials_provider(creds)
            .load()
            .await;

        Ok(Self {
            client: Client::new(&config),
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
            bucket: bucket.to_string(),
            region: region.to_string(),
            local_dir: PathBuf::from(local_dir),
            down_dir: PathBuf::from(down_dir),
            use_local,
        })
    }

    /// ファイルをアップロードし、保存先のキーを返します (Go: Up)
    pub async fn up(&self, file_path: &str) -> Result<String> {
        let now = Local::now();
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
        } else {
            self.ensure_s3_settings()?;
            let body = aws_sdk_s3::primitives::ByteStream::from_path(file_path).await?;
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(&full_key)
                .body(body)
                .send()
                .await?;
        }
        Ok(full_key)
    }

    /// ファイルをDLし、ローカルのキャッシュパスを返します (Go: Down)
    pub async fn down(&self, path_from_up: &str) -> Result<PathBuf> {
        let clean_key = path_from_up.trim_start_matches('/');
        let cache_path = self.down_dir.join(clean_key);

        // 1. キャッシュ済みなら即座に返却 (NotFound以外のエラーは即時返却)
        match fs::metadata(&cache_path).await {
            Ok(_) => return Ok(cache_path),
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => (),
        }

        // 2. local_dir（アップロード済み実体）を確認
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
            return Err(anyhow!("File not found."));
        }

        // 3. S3から取得してキャッシュに保存
        self.ensure_s3_settings()?;
        let resp = self.client.get_object().bucket(&self.bucket).key(clean_key).send().await?;
        fs::create_dir_all(cache_path.parent().unwrap()).await?;
        let mut file = fs::File::create(&cache_path).await?;
        let mut body_reader = resp.body.into_async_read();
        tokio::io::copy(&mut body_reader, &mut file).await?;

        Ok(cache_path)
    }

    /// ファイルを削除し、空になった親ディレクトリを掃除します (Go: Del)
    pub async fn del(&self, path_from_up: &str) -> Result<()> {
        let clean_key = path_from_up.trim_start_matches('/');
        let mut local_err: Option<anyhow::Error> = None;
        let mut s3_err = None;

        // 1. ダウンロードキャッシュの削除
        let cache_path = self.down_dir.join(clean_key);
        match fs::metadata(&cache_path).await {
            Ok(_) => {
                if let Err(e) = fs::remove_file(&cache_path).await {
                    return Err(anyhow!("Failed to delete local-down-cache-file '{:?}': {}", cache_path, e));
                }
                let _ = self.tidy_up_dirs(&self.down_dir, cache_path.parent().unwrap()).await;
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(anyhow!("Failed to access local-down-cache-file '{:?}': {}", cache_path, e));
            }
            _ => (),
        }

        // 2. ローカル実体の削除
        let local_path = self.local_dir.join(clean_key);
        match fs::metadata(&local_path).await {
            Ok(_) => {
                if let Err(e) = fs::remove_file(&local_path).await {
                    println!("Failed to delete local file: {:?}", local_path);
                    local_err = Some(e.into());
                } else {
                    let _ = self.tidy_up_dirs(&self.local_dir, local_path.parent().unwrap()).await;
                }
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                println!("Failed to access local file: {:?}", local_path);
                local_err = Some(e.into());
            }
            _ => (),
        }

        // 3. S3から削除
        if !self.use_local {
            if !self.is_valid_s3_settings() {
                s3_err = Some(anyhow!("Invalid S3 settings."));
            } else {
                // HeadObjectで存在確認
                let head_res = self.client.head_object().bucket(&self.bucket).key(clean_key).send().await;
                match head_res {
                    Ok(_) => {
                        if let Err(e) = self.client.delete_object().bucket(&self.bucket).key(clean_key).send().await {
                            println!("Failed to delete S3 object: {}", clean_key);
                            s3_err = Some(e.into());
                        }
                    }
                    Err(e) => {
                        let is_not_found = if let Some(service_err) = e.as_service_error() {
                            service_err.is_not_found()
                        } else {
                            false
                        };
                        if !is_not_found {
                            s3_err = Some(e.into());
                        }
                    }
                }
            }
        }

        // エラー統合
        match (local_err, s3_err) {
            (Some(le), Some(se)) => Err(anyhow!("Failed to delete file locally and from S3: local error: {:?}, S3 error: {:?}", le, se)),
            (Some(le), None) => Err(anyhow!("Failed to delete file locally: {:?}", le)),
            (None, Some(se)) => Err(anyhow!("Failed to delete file from S3: {:?}", se)),
            (None, None) => Ok(()),
        }
    }

    /// 指定の正規表現にマッチするオブジェクトを走査します (Go: Walk)
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
        if self.use_local {
            for entry in walkdir::WalkDir::new(&self.local_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    if re.is_match(&filename) {
                        let rel_path = entry.path().strip_prefix(&self.local_dir)?.to_string_lossy().to_string();
                        callback(rel_path, filename).await?;
                        if interval_ms > 0 { tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await; }
                    }
                }
            }
        } else {
            self.ensure_s3_settings()?;
            let mut paginator = self.client.list_objects_v2().bucket(&self.bucket).into_paginator().send();
            while let Some(page) = paginator.next().await {
                for obj in page?.contents() {
                    let key = obj.key().unwrap_or_default().to_string();
                    let filename = Path::new(&key).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
                    if re.is_match(&filename) {
                        callback(key, filename).await?;
                        if interval_ms > 0 { tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await; }
                    }
                }
            }
        }

        if let Some(cb) = final_callback {
            cb().await?;
        }
        Ok(())
    }

    /// ファイルの存在確認 (Go: IsExist)
    pub async fn is_exist(&self, path_from_up: &str) -> bool {
        let clean_key = path_from_up.trim_start_matches('/');
        if fs::metadata(self.local_dir.join(clean_key)).await.is_ok() { return true; }
        if self.use_local || !self.is_valid_s3_settings() { return false; }

        self.client.head_object().bucket(&self.bucket).key(clean_key).send().await.is_ok()
    }

    /// 再帰的に空のディレクトリを削除する内部関数
    async fn tidy_up_dirs(&self, root: &Path, start: &Path) -> Result<()> {
        let mut current = start;
        while current != root && current.to_str() != Some("/") {
            let mut entries = fs::read_dir(current).await?;
            if entries.next_entry().await?.is_none() {
                fs::remove_dir(current).await?;
                current = current.parent().ok_or_else(|| anyhow!("Root reached"))?;
            } else { break; }
        }
        Ok(())
    }

    pub fn is_valid_s3_settings(&self) -> bool {
        let e = "empty";
        ![&self.access_key, &self.secret_key, &self.bucket, &self.region].iter().any(|&s| s == e)
    }

    fn ensure_s3_settings(&self) -> Result<()> {
        if self.is_valid_s3_settings() { Ok(()) } else { Err(anyhow!("Invalid S3 settings.")) }
    }

    /// S3 への接続と権限を確認します。
    /// ローカルモードの場合はディレクトリの存在を確認します。
    pub async fn validate_connection(&self) -> Result<()> {
        if self.use_local {
            if fs::metadata(&self.local_dir).await.is_ok() {
                Ok(())
            } else {
                Err(anyhow!("Local storage directory does not exist: {:?}", self.local_dir))
            }
        } else {
            self.ensure_s3_settings()?;
            // リストの取得を試みて権限を確認（最大1件取得）
            self.client
                .list_objects_v2()
                .bucket(&self.bucket)
                .max_keys(1)
                .send()
                .await
                .map(|_| ())
                .map_err(|e| anyhow!("Failed to validate S3 connection: {}", e))
        }
    }
}
