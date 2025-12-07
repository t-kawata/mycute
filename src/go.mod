module mycute

go 1.25.3

require (
	github.com/aws/aws-sdk-go-v2 v1.36.3
	github.com/aws/aws-sdk-go-v2/config v1.29.4
	github.com/aws/aws-sdk-go-v2/credentials v1.17.57
	github.com/aws/aws-sdk-go-v2/service/s3 v1.78.2
	github.com/aws/smithy-go v1.22.2
	github.com/google/uuid v1.6.0
	github.com/ikawaha/kagome-dict/ipa v1.2.5
	github.com/ikawaha/kagome/v2 v2.10.2
	github.com/joho/godotenv v1.5.1
	github.com/kuzudb/go-kuzu v0.11.3
	github.com/pkoukk/tiktoken-go v0.1.6
	github.com/tmc/langchaingo v0.1.14
	golang.org/x/sync v0.16.0
)

require (
	github.com/aws/aws-sdk-go-v2/aws/protocol/eventstream v1.6.10 // indirect
	github.com/aws/aws-sdk-go-v2/feature/ec2/imds v1.16.27 // indirect
	github.com/aws/aws-sdk-go-v2/internal/configsources v1.3.34 // indirect
	github.com/aws/aws-sdk-go-v2/internal/endpoints/v2 v2.6.34 // indirect
	github.com/aws/aws-sdk-go-v2/internal/ini v1.8.2 // indirect
	github.com/aws/aws-sdk-go-v2/internal/v4a v1.3.34 // indirect
	github.com/aws/aws-sdk-go-v2/service/internal/accept-encoding v1.12.3 // indirect
	github.com/aws/aws-sdk-go-v2/service/internal/checksum v1.7.0 // indirect
	github.com/aws/aws-sdk-go-v2/service/internal/presigned-url v1.12.15 // indirect
	github.com/aws/aws-sdk-go-v2/service/internal/s3shared v1.18.15 // indirect
	github.com/aws/aws-sdk-go-v2/service/sso v1.24.14 // indirect
	github.com/aws/aws-sdk-go-v2/service/ssooidc v1.28.13 // indirect
	github.com/aws/aws-sdk-go-v2/service/sts v1.33.12 // indirect
	github.com/dlclark/regexp2 v1.10.0 // indirect
	github.com/ikawaha/kagome-dict v1.1.6 // indirect
	github.com/shopspring/decimal v1.4.0 // indirect
	github.com/stretchr/testify v1.11.1 // indirect
)

replace github.com/kuzudb/go-kuzu => github.com/t-kawata/go-kuzu v0.0.0-20251010145220-3950bb8051f9
