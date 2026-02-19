# Rust版ストレージ抽象化モジュール (S3/Local) 開発指示書

本ドキュメントは、Go言語で実装されたS3/Local抽象化クライアントを、Rust（Tokio + AWS SDK v1）環境へ完全に移植するための手順書です。

## 0. 元のGo版のコード
```go
package s3client

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	"github.com/aws/smithy-go"
)

// S3Client wraps an S3 client plus config for local storage.
type S3Client struct {
	client    *s3.Client
	accessKey string
	secretKey string
	bucket    string
	region    string
	localDir  string
	downDir   string
	useLocal  bool
}

type WalkFunc func(path string, filename string) error

// NewS3Client creates a new S3Client with AWS SDK v2.
// Always initializes the S3 client for better flexibility and future extensibility.
func NewS3Client(accessKey, secretKey, region, bucket, localDir string, downDir string, useLocal bool) (*S3Client, error) {
	if accessKey == "" || secretKey == "" || region == "" || bucket == "" || localDir == "" {
		return nil, errors.New("Invalid args.")
	}

	// Always create S3 client for better flexibility, even in local mode
	cfg, err := config.LoadDefaultConfig(context.TODO(),
		config.WithRegion(region),
		config.WithCredentialsProvider(credentials.NewStaticCredentialsProvider(accessKey, secretKey, "")),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to load AWS config: %w", err)
	}

	s3Client := s3.NewFromConfig(cfg)

	return &S3Client{
		client:    s3Client,
		accessKey: accessKey,
		secretKey: secretKey,
		bucket:    bucket,
		region:    region,
		localDir:  localDir,
		downDir:   downDir,
		useLocal:  useLocal,
	}, nil
}

// Up uploads a local file either to the localDir (if useLocal=true) or to S3.
// Returns the relative path (key) under which the file was saved.
func (c *S3Client) Up(filePath string) (*string, error) {
	currentTime := time.Now()
	dir := fmt.Sprintf("%d/%02d/%02d_%02d-%02d",
		currentTime.Year(), currentTime.Month(), currentTime.Day(),
		currentTime.Hour(), currentTime.Minute())
	fileName := filepath.Base(filePath)

	if c.useLocal {
		destDir := filepath.Join(c.localDir, dir)
		if err := os.MkdirAll(destDir, os.ModePerm); err != nil {
			return nil, err
		}
		destPath := filepath.Join(destDir, fileName)

		inputFile, err := os.Open(filePath)
		if err != nil {
			return nil, err
		}
		defer inputFile.Close()

		outputFile, err := os.Create(destPath)
		if err != nil {
			return nil, err
		}
		defer outputFile.Close()

		_, err = io.Copy(outputFile, inputFile)
		if err != nil {
			return nil, err
		}

		pathStr := filepath.Join(dir, fileName)
		return aws.String(pathStr), nil
	} else {
		if !c.IsValidS3Settings() {
			return nil, errors.New("Invalid S3 settings.")
		}

		fullKey := filepath.Join(dir, fileName)
		file, err := os.Open(filePath)
		if err != nil {
			return nil, err
		}
		defer file.Close()

		// Use ReadFrom for better performance (maintains v1 behavior)
		buf := new(bytes.Buffer)
		_, err = buf.ReadFrom(file)
		if err != nil {
			return nil, err
		}

		// Use context with timeout for better error handling
		ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
		defer cancel()

		_, err = c.client.PutObject(ctx, &s3.PutObjectInput{
			Bucket: aws.String(c.bucket),
			Key:    aws.String(fullKey),
			Body:   bytes.NewReader(buf.Bytes()),
		})
		if err != nil {
			return nil, err
		}

		return aws.String(fullKey), nil
	}
}

// ファイルをDLしてローカルのパスを返す
func (c *S3Client) Down(pathFromUp string) (*string, error) {
	pathFromUp = strings.TrimPrefix(pathFromUp, "/")
	localFilePath := filepath.Join(c.localDir, pathFromUp)
	toFilePath := filepath.Join(c.downDir, pathFromUp)

	if _, err := os.Stat(toFilePath); err == nil {
		return &toFilePath, nil // 既にあるならパスを返して終わり
	} else if !os.IsNotExist(err) {
		return nil, err // os.Statで予期しないエラーがあった場合はそのまま返す
	}

	// Try local first
	inputFile, err := os.Open(localFilePath)
	if err == nil { // ローカルで見つかったら
		defer inputFile.Close()
		err := os.MkdirAll(filepath.Dir(toFilePath), 0755)
		if err != nil {
			return nil, err
		}
		outputFile, err := os.Create(toFilePath)
		if err != nil {
			return nil, err
		}
		defer outputFile.Close()
		_, err = io.Copy(outputFile, inputFile)
		if err != nil {
			return nil, err
		}
		return &toFilePath, nil
	}

	// ローカルでは見つからなければ
	if c.useLocal {
		return nil, errors.New("File not found.")
	}

	if !c.IsValidS3Settings() {
		return nil, errors.New("Invalid S3 settings.")
	}

	// Use context with timeout
	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	output, err := c.client.GetObject(ctx, &s3.GetObjectInput{
		Bucket: aws.String(c.bucket),
		Key:    aws.String(pathFromUp),
	})
	if err != nil {
		return nil, err
	}
	defer output.Body.Close()

	err = os.MkdirAll(filepath.Dir(toFilePath), 0755)
	if err != nil {
		return nil, err
	}
	file, err := os.Create(toFilePath)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	_, err = io.Copy(file, output.Body)
	if err != nil {
		return nil, err
	}
	return &toFilePath, nil
}

// Del deletes a file identified by pathFromUp both locally and (if useLocal=false) on S3.
func (c *S3Client) Del(pathFromUp string) error {
	var localErr, s3Err error
	pathFromUp = strings.TrimPrefix(pathFromUp, "/")
	localFilePath := filepath.Join(c.localDir, pathFromUp)
	downLocalCacheFilePath := filepath.Join(c.downDir, pathFromUp)

	if _, err := os.Stat(downLocalCacheFilePath); err == nil { // ダウンロードでローカルにキャッシュしたファイルがある時は削除
		downLocalCacheFileErr := os.Remove(downLocalCacheFilePath)
		if downLocalCacheFileErr != nil {
			return fmt.Errorf("Failed to delete local-down-cache-file '%s': %s\n", downLocalCacheFilePath, downLocalCacheFileErr.Error())
		} else {
			// ファイル削除後、空ディレクトリ掃除（c.downDirは絶対に消さない）
			dir := filepath.Dir(downLocalCacheFilePath)
			for dir != c.downDir && dir != "." && dir != "/" {
				files, err := os.ReadDir(dir)
				if err != nil {
					break
				}
				if len(files) > 0 {
					break
				}
				os.Remove(dir) // 空ディレクトリを削除
				dir = filepath.Dir(dir)
			}
		}
	}

	// ローカルファイルの削除
	if _, err := os.Stat(localFilePath); err == nil {
		localErr = os.Remove(localFilePath)
		if localErr != nil {
			fmt.Printf("Failed to delete local file: %s\n", localFilePath)
		} else {
			// ファイル削除後、空ディレクトリ掃除（c.localDirは絶対に消さない）
			dir := filepath.Dir(localFilePath)
			for dir != c.localDir && dir != "." && dir != "/" {
				files, err := os.ReadDir(dir)
				if err != nil {
					break
				}
				if len(files) > 0 {
					break
				}
				os.Remove(dir) // 空ディレクトリを削除
				dir = filepath.Dir(dir)
			}
		}
	} else if !errors.Is(err, os.ErrNotExist) {
		localErr = err
	}

	// S3ファイルの削除 (useLocal=falseのとき)
	if !c.useLocal {
		if !c.IsValidS3Settings() {
			s3Err = errors.New("Invalid S3 settings.")
		} else {
			// Use context with timeout
			ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
			defer cancel()

			_, err := c.client.HeadObject(ctx, &s3.HeadObjectInput{
				Bucket: aws.String(c.bucket),
				Key:    aws.String(pathFromUp),
			})
			if err == nil {
				_, s3Err = c.client.DeleteObject(ctx, &s3.DeleteObjectInput{
					Bucket: aws.String(c.bucket),
					Key:    aws.String(pathFromUp),
				})
				if s3Err != nil {
					fmt.Printf("Failed to delete S3 object: %s\n", pathFromUp)
				}
			} else {
				var apiErr smithy.APIError
				if errors.As(err, &apiErr) && apiErr.ErrorCode() == "NoSuchKey" {
					// S3上に存在しないので削除不要
					s3Err = nil
				} else {
					s3Err = err
				}
			}
		}
	}

	// エラー統合
	if localErr != nil && s3Err != nil {
		return fmt.Errorf("Failed to delete file locally and from S3: local error: %v, S3 error: %v", localErr, s3Err)
	} else if localErr != nil {
		return fmt.Errorf("Failed to delete file locally: %v", localErr)
	} else if s3Err != nil {
		return fmt.Errorf("Failed to delete file from S3: %v", s3Err)
	}

	return nil
}

func (c *S3Client) Walk(re *regexp.Regexp, callback WalkFunc, callbackOperateIntervalMs int, finalCallback func() error) error {
	if c.useLocal {
		err := filepath.WalkDir(c.localDir, func(path string, d fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if !d.Type().IsRegular() {
				return nil
			}
			filename := d.Name()
			if re.MatchString(filename) {
				relPath, err := filepath.Rel(c.localDir, path)
				if err != nil {
					return err
				}
				err = callback(relPath, filename)
				if err != nil {
					return err
				}
				if callbackOperateIntervalMs > 0 {
					time.Sleep(time.Duration(callbackOperateIntervalMs) * time.Millisecond)
				}
			}
			return nil
		})
		if finalCallback != nil {
			if cbErr := finalCallback(); cbErr != nil {
				return cbErr
			}
		}
		return err
	} else {
		if !c.IsValidS3Settings() {
			return errors.New("Invalid S3 settings.")
		}
		ctx := context.Background()
		paginator := s3.NewListObjectsV2Paginator(c.client, &s3.ListObjectsV2Input{
			Bucket: aws.String(c.bucket),
		})
		for paginator.HasMorePages() {
			output, err := paginator.NextPage(ctx)
			if err != nil {
				return err
			}
			for _, obj := range output.Contents {
				key := aws.ToString(obj.Key)
				filename := filepath.Base(key)
				if re.MatchString(filename) {
					err := callback(key, filename)
					if err != nil {
						return err
					}
					if callbackOperateIntervalMs > 0 {
						time.Sleep(time.Duration(callbackOperateIntervalMs) * time.Millisecond)
					}
				}
			}
		}
		if finalCallback != nil {
			if cbErr := finalCallback(); cbErr != nil {
				return cbErr
			}
		}
		return nil
	}
}

// IsExist returns true if the file identified by pathFromUp exists in localDir or S3.
func (c *S3Client) IsExist(pathFromUp string) bool {
	pathFromUp = strings.TrimPrefix(pathFromUp, "/")
	localFilePath := filepath.Join(c.localDir, pathFromUp)
	// まずローカルを確認
	if _, err := os.Stat(localFilePath); err == nil {
		return true
	}
	// ローカルのみ利用の場合はfalse
	if c.useLocal {
		return false
	}
	// S3の設定が正しいか確認
	if !c.IsValidS3Settings() {
		return false
	}
	// S3で存在チェック（HEAD Object）
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	_, err := c.client.HeadObject(ctx, &s3.HeadObjectInput{
		Bucket: aws.String(c.bucket),
		Key:    aws.String(pathFromUp),
	})
	if err == nil {
		return true
	}
	var apiErr smithy.APIError
	if errors.As(err, &apiErr) && apiErr.ErrorCode() == "NotFound" {
		return false
	}
	return false // その他エラーも「存在しない」とみなす
}

// IsValidS3Settings returns false if any of the key settings are equal to "empty".
func (c *S3Client) IsValidS3Settings() bool {
	empty := "empty"
	if c.accessKey == empty || c.secretKey == empty || c.bucket == empty || c.region == empty {
		return false
	}
	return true
}
```

## 1. 依存関係のセットアップ
`Cargo.toml`を直接編集する代わりに、以下のコマンドを実行して最新の安定版パッケージを導入してください。

```bash
# 非同期ランタイムとエラーハンドリング
cargo add tokio --features full
cargo add anyhow

# AWS SDK v1 (最新の BehaviorVersion に対応)
cargo add aws-config
cargo add aws-sdk-s3 --features rt-tokio
cargo add aws-credential-types
cargo add aws-smithy-runtime-api

# ファイル・パス操作、ユーティリティ
cargo add walkdir
cargo add chrono
cargo add regex
cargo add futures
```

***

## 2. 実装コード: `src/utils/s3_client.rs`
以下のコードを `src/utils/s3_client.rs` として保存してください。Go版のロジックを忠実に再現しつつ、Rustのメモリ安全性能と非同期処理（Async/Await）を最適化しています。

```rust
use anyhow::{anyhow, Result};
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use chrono::Local;
use futures::StreamExt;
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
        let mut local_err = None;
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
}
```

***

## 3. 実装上の変更理由と注意点

1.  **非同期処理の導入**:
    RustではI/O操作（ファイル操作、S3通信）を `tokio` ランタイムで非同期化する必要があります。すべてのメソッドを `async fn` とし、`await` で待機するように変更しました。

2.  **メモリ効率の最適化 (`up` メソッド)**:
    Go版では `bytes.Buffer` にファイルを一度すべて読み込んでいましたが、Rust版では `ByteStream::from_path` を使用しています。これにより、大容量ファイルでもメモリを消費せず、ストリーミングでアップロードされます。

3.  **ディレクトリクリーンアップ (`tidy_up_dirs`)**:
    Rustの所有権システムに適合させるため、ディレクトリの再帰削除ロジックを独立した内部関数として分離しました。

4.  **エラーハンドリング**:
    `anyhow::Result` を採用することで、ファイルシステムのエラー、ネットワークエラー、バリデーションエラーを一つの型で透過的に扱えるようにしました。