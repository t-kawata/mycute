package common

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"math/rand"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"regexp"
	"slices"
	"strconv"
	"strings"
	"syscall"
	"time"
	"unicode/utf8"

	"github.com/google/uuid"
	"github.com/t-kawata/mycute/lib/httpclient"
)

func Exec(name string, arg ...string) (result string, err error) {
	result = ""
	cmd := exec.Command(name, arg...)
	var out bytes.Buffer
	cmd.Stdout = &out
	err = cmd.Run()
	if err != nil {
		return
	}
	result = out.String()
	return
}

func StrToInt(strNum string) int {
	if r, err := strconv.ParseInt(strNum, 10, 64); err == nil {
		return int(r)
	}
	return 0
}

func StrToInt32(strNum string) int32 {
	if r, err := strconv.ParseInt(strNum, 10, 32); err == nil {
		return int32(r)
	}
	return 0
}

func StrToFloat32(strNum string) float32 {
	if r, err := strconv.ParseFloat(strNum, 32); err == nil {
		return float32(r)
	}
	return 0
}

func StrToFloat64(strNum string) float64 {
	if r, err := strconv.ParseFloat(strNum, 64); err == nil {
		return float64(r)
	}
	return 0
}

func StrToUint(strNum string) uint {
	if r, err := strconv.ParseUint(strNum, 10, 64); err == nil {
		return uint(r)
	}
	return 0
}

func StrToUint8(strNum string) uint8 {
	if r, err := strconv.ParseUint(strNum, 10, 8); err == nil {
		return uint8(r)
	}
	return 0
}

func StrToUint16(strNum string) uint16 {
	if r, err := strconv.ParseUint(strNum, 10, 16); err == nil {
		return uint16(r)
	}
	return 0
}

func IsEmpty(val any) bool {
	if val == nil {
		return true
	}
	if IsNumeric(val) {
		return false
	}
	switch v := reflect.ValueOf(val); v.Kind() {
	case reflect.Array, reflect.Map, reflect.Slice, reflect.String:
		return v.Len() == 0
	case reflect.Bool:
		return false
	default:
		return reflect.DeepEqual(val, reflect.Zero(reflect.TypeOf(val)).Interface())
	}
}

func IsNumeric(val any) bool {
	switch val.(type) {
	case int, int8, int16, int32, int64,
		uint, uint8, uint16, uint32, uint64, uintptr,
		float32, float64, complex64, complex128:
		return true
	default:
		return false
	}
}

func CountUTF8Chars(s string) int {
	return utf8.RuneCountInString(s)
}

func ExecOsBash(command string) (*string, error) {
	cmd := exec.Command("bash", "-c", command)
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	err := cmd.Run()
	if err != nil {
		return nil, fmt.Errorf("command execution error: %v, stderr: %s", err, stderr.String())
	}
	r := strings.TrimSpace(stdout.String())
	return &r, nil
}

func GetMyPublicIP() (*string, error) {
	ip, err := ExecOsBash("public")
	if err != nil {
		return nil, err
	}
	return ip, nil
}

func GetMyPrivateIP() (*string, error) {
	ip, err := ExecOsBash("private")
	if err != nil {
		return nil, err
	}
	return ip, nil
}

func GenUUID() *string {
	uuid := uuid.New().String()
	return &uuid
}

func RandStr(n uint8) string {
	const letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!?-_*#/"
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	b := make([]byte, n)
	for i := range b {
		b[i] = letters[r.Intn(len(letters))]
	}
	return string(b)
}

func RandStrAlphaNum(n uint8) string {
	const letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	b := make([]byte, n)
	for i := range b {
		b[i] = letters[r.Intn(len(letters))]
	}
	return string(b)
}

func ConvertAllWhiteToSingleSpace(input *string) {
	*input = strings.ReplaceAll(*input, "\n", " ")
	regex := regexp.MustCompile(`[ \t]{2,}`)
	*input = regex.ReplaceAllString(*input, " ")
}

func TOpe[T any](check bool, ifTrue T, ifFalse T) T {
	if check {
		return ifTrue
	} else {
		return ifFalse
	}
}

func GetDiskFreeSpaceMB(path string) (uint64, error) {
	var stat syscall.Statfs_t
	err := syscall.Statfs(path, &stat)
	if err != nil {
		return 0, err
	}
	freeMB := (stat.Bavail * uint64(stat.Bsize)) / (1024 * 1024)
	return freeMB, nil
}

// ディレクトリを再帰的に走査して、指定した時間を経過しているファイルだけ callback に渡す
func WalkAndFindTimeoverFiles(rootDirPath string, minutes int, callback func(filePath string, elapsedMinutes int)) error {
	cutoff := time.Now().Add(-time.Duration(minutes) * time.Minute)
	now := time.Now()
	return filepath.Walk(rootDirPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.Mode().IsRegular() {
			if info.ModTime().Before(cutoff) {
				elapsed := int(now.Sub(info.ModTime()).Minutes())
				callback(path, elapsed)
			}
		}
		return nil
	})
}

// ディレクトリを再帰的に走査して、空のディレクトリだけ callback に渡す
func WalkAndFindEmptyDirs(rootDirPath string, callback func(dirPath string)) error {
	var walk func(path string) (bool, error)
	walk = func(path string) (bool, error) {
		entries, err := os.ReadDir(path)
		if err != nil {
			return false, err
		}
		empty := true
		for _, entry := range entries {
			if entry.IsDir() {
				childPath := filepath.Join(path, entry.Name())
				childEmpty, err := walk(childPath)
				if err != nil {
					return false, err
				}
				if !childEmpty {
					empty = false
				}
			} else {
				empty = false
			}
		}
		if path != rootDirPath && empty {
			callback(path)
		}
		return empty, nil
	}
	_, err := walk(rootDirPath)
	return err
}

func IsValidRegex(pattern string) bool {
	_, err := regexp.Compile(pattern)
	return err == nil
}

func IsValidCttsHostIdent(hc *httpclient.HttpClient, cttsHost string, cttsIdent string) bool {
	url := fmt.Sprintf("https://ctts.%s/models", cttsHost)
	body, code, err := hc.GetWithHeaders(&url, nil)
	if err != nil {
		return false
	}
	if code == nil || *code != 200 {
		return false
	}
	if body == nil {
		return false
	}
	var models []string
	if err := json.Unmarshal([]byte(*body), &models); err != nil {
		return false
	}
	return slices.Contains(models, cttsIdent)
}

func FormatJapaneseName(name string) (string, error) {
	// Check if name contains space (half-width or full-width)
	if !strings.Contains(name, " ") && !strings.Contains(name, "　") {
		return "", fmt.Errorf("name must contain a space")
	}

	// Replace full-width space with half-width space
	name = strings.ReplaceAll(name, "　", " ")

	// Convert multiple spaces to single space and trim
	ConvertAllWhiteToSingleSpace(&name)
	name = strings.TrimSpace(name)

	// Split by space
	parts := strings.Split(name, " ")
	if len(parts) < 2 {
		// Should not happen if validation passed, but for safety
		return name, nil
	}

	// Join with single space, ensuring LastName comes first (index 0)
	// The requirement says: "index: 0 の要素をラストネーム、それ以降をファーストネームと扱い、ラストネームとファーストネームをラストネームが最初になるように半角スペースで連結"
	// This effectively means just joining them back with a space, which is what we have if we just trim and single-space it.
	// But let's be explicit about the reconstruction if needed.
	// Actually, ConvertAllWhiteToSingleSpace + TrimSpace already does the job of "converting all spaces to single half-width space and removing leading/trailing".
	// The only specific logic is "index 0 is last name, rest is first name".
	// If the input was " Tanaka   Taro ", it becomes "Tanaka Taro".
	// If input was "Tanaka  Taro  Jiro", it becomes "Tanaka Taro Jiro".
	// The requirement implies we should treat parts[0] as Last Name and strings.Join(parts[1:], " ") as First Name.
	// And then join them as "LastName FirstName".
	// This results in the same string as just joining all parts with " ".

	return name, nil
}

func CalculateSHA256(data []byte) string {
	hash := sha256.Sum256(data)
	return hex.EncodeToString(hash[:])
}
