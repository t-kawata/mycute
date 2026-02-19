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
	if localDir == "" {
		return nil, errors.New("localDir is required")
	}
	if downDir == "" {
		return nil, errors.New("downDir is required")
	}

	if !useLocal {
		if accessKey == "" || secretKey == "" || region == "" || bucket == "" {
			return nil, errors.New("AWS credentials and bucket are required when useLocal is false")
		}
	} else {
		// In local mode, if AWS args are missing, use dummies to prevent initialization errors
		if accessKey == "" {
			accessKey = "dummy"
		}
		if secretKey == "" {
			secretKey = "dummy"
		}
		if region == "" {
			region = "us-east-1"
		}
		if bucket == "" {
			bucket = "dummy"
		}
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

// CleanupDownDir removes files in the download directory that are older than the specified retention period.
// It walks through the downDir and deletes files with modification time older than time.Now() - retention.
func (c *S3Client) CleanupDownDir(retention time.Duration) error {
	if c.downDir == "" {
		return nil
	}

	threshold := time.Now().Add(-retention)
	fmt.Printf("S3Client: Cleaning up download directory %s (older than %v)\n", c.downDir, retention)

	err := filepath.WalkDir(c.downDir, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if !d.Type().IsRegular() {
			return nil
		}

		info, err := d.Info()
		if err != nil {
			return nil // Skip if info cannot be retrieved
		}

		if info.ModTime().Before(threshold) {
			if err := os.Remove(path); err != nil {
				fmt.Printf("S3Client: Failed to remove old file %s: %v\n", path, err)
			} else {
				fmt.Printf("S3Client: Removed old file %s\n", path)
			}
		}
		return nil
	})

	// Clean up empty directories
	_ = filepath.WalkDir(c.downDir, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return nil
		}
		if d.IsDir() && path != c.downDir {
			entries, _ := os.ReadDir(path)
			if len(entries) == 0 {
				_ = os.Remove(path)
			}
		}
		return nil
	})

	return err
}
