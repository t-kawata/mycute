package httpclient

import (
	"bytes"
	"crypto/tls"
	"io"
	"net/http"

	"github.com/t-kawata/mycute/config"
	"go.uber.org/zap"
)

const postContentType = "application/json; charset=utf-8"

type HttpClient struct {
	Client *http.Client
	Logger *zap.Logger
	Env    *config.Env
}

func NewDefaultHttpClient(l *zap.Logger, env *config.Env) *HttpClient {
	// return &HttpClient{Logger: l, Env: env, Client: &http.Client{
	// 	Transport: http.DefaultTransport, // HTTP/2 or HTTP/1.1
	// }}
	transport := &http.Transport{ // HTTP/1.1
		TLSNextProto: make(map[string]func(authority string, c *tls.Conn) http.RoundTripper),
	}
	return &HttpClient{Logger: l, Env: env, Client: &http.Client{
		Transport: transport,
	}}
}

func (hc *HttpClient) Get(url *string) (body *string, code *int, err error) {
	res, err := hc.Client.Get(*url)
	if err != nil {
		return
	}
	defer res.Body.Close()
	bodyBytes, err := io.ReadAll(res.Body)
	if err != nil {
		return
	}
	bodyBase := string(bodyBytes)
	body = &bodyBase
	code = &res.StatusCode
	return
}

func (hc *HttpClient) Post(url *string, jsonStr *string) (body *string, code *int, err error) {
	res, err := hc.Client.Post(*url, postContentType, bytes.NewBuffer([]byte(*jsonStr)))
	if err != nil {
		return
	}
	defer res.Body.Close()
	bodyBytes, err := io.ReadAll(res.Body)
	if err != nil {
		return
	}
	bodyBase := string(bodyBytes)
	body = &bodyBase
	code = &res.StatusCode
	return
}

func (hc *HttpClient) Put(url *string, jsonStr *string) (body *string, code *int, err error) {
	res, err := hc.Client.Post(*url, postContentType, bytes.NewBuffer([]byte(*jsonStr)))
	if err != nil {
		return
	}
	defer res.Body.Close()
	bodyBytes, err := io.ReadAll(res.Body)
	if err != nil {
		return
	}
	bodyBase := string(bodyBytes)
	body = &bodyBase
	code = &res.StatusCode
	return
}

func (hc *HttpClient) Patch(url *string, jsonStr *string) (body *string, code *int, err error) {
	req, err := http.NewRequest(http.MethodPatch, *url, bytes.NewBuffer([]byte(*jsonStr)))
	if err != nil {
		return
	}
	req.Header.Set("Content-Type", postContentType)
	res, err := hc.Client.Do(req)
	if err != nil {
		return
	}
	defer res.Body.Close()
	bodyBytes, err := io.ReadAll(res.Body)
	if err != nil {
		return
	}
	bodyBase := string(bodyBytes)
	body = &bodyBase
	code = &res.StatusCode
	return
}

func (hc *HttpClient) Delete(url *string) (body *string, code *int, err error) {
	req, err := http.NewRequest(http.MethodDelete, *url, nil)
	if err != nil {
		return
	}
	res, err := hc.Client.Do(req)
	if err != nil {
		return
	}
	defer res.Body.Close()
	bodyBytes, err := io.ReadAll(res.Body)
	if err != nil {
		return
	}
	bodyBase := string(bodyBytes)
	body = &bodyBase
	code = &res.StatusCode
	return
}

func (hc *HttpClient) GetWithHeaders(url *string, headers *map[string]string) (body *string, code *int, err error) {
	req, err := http.NewRequest("GET", *url, nil)
	if err != nil {
		return
	}
	if headers != nil {
		for key, value := range *headers {
			req.Header.Set(key, value)
		}
	}
	res, err := hc.Client.Do(req)
	if err != nil {
		return
	}
	defer res.Body.Close()
	bodyBytes, err := io.ReadAll(res.Body)
	if err != nil {
		return
	}
	bodyBase := string(bodyBytes)
	body = &bodyBase
	code = &res.StatusCode
	return
}

func (hc *HttpClient) PostWithHeaders(url *string, jsonStr *string, headers *map[string]string) (body *string, code *int, err error) {
	req, err := http.NewRequest("POST", *url, bytes.NewBuffer([]byte(*jsonStr)))
	if err != nil {
		return
	}
	req.Header.Set("Content-Type", postContentType)
	if headers != nil {
		for key, value := range *headers {
			req.Header.Set(key, value)
		}
	}
	res, err := hc.Client.Do(req)
	if err != nil {
		return
	}
	defer res.Body.Close()
	bodyBytes, err := io.ReadAll(res.Body)
	if err != nil {
		return
	}
	bodyBase := string(bodyBytes)
	body = &bodyBase
	code = &res.StatusCode
	return
}

func (hc *HttpClient) PutWithHeaders(url *string, jsonStr *string, headers *map[string]string) (body *string, code *int, err error) {
	req, err := http.NewRequest(http.MethodPut, *url, bytes.NewBuffer([]byte(*jsonStr)))
	if err != nil {
		return
	}
	req.Header.Set("Content-Type", postContentType)
	if headers != nil {
		for key, value := range *headers {
			req.Header.Set(key, value)
		}
	}
	res, err := hc.Client.Do(req)
	if err != nil {
		return
	}
	defer res.Body.Close()
	bodyBytes, err := io.ReadAll(res.Body)
	if err != nil {
		return
	}
	bodyBase := string(bodyBytes)
	body = &bodyBase
	code = &res.StatusCode
	return
}

func (hc *HttpClient) PatchWithHeaders(url *string, jsonStr *string, headers *map[string]string) (body *string, code *int, err error) {
	req, err := http.NewRequest(http.MethodPatch, *url, bytes.NewBuffer([]byte(*jsonStr)))
	if err != nil {
		return
	}
	req.Header.Set("Content-Type", postContentType)
	if headers != nil {
		for key, value := range *headers {
			req.Header.Set(key, value)
		}
	}
	res, err := hc.Client.Do(req)
	if err != nil {
		return
	}
	defer res.Body.Close()
	bodyBytes, err := io.ReadAll(res.Body)
	if err != nil {
		return
	}
	bodyBase := string(bodyBytes)
	body = &bodyBase
	code = &res.StatusCode
	return
}

func (hc *HttpClient) DeleteWithHeaders(url *string, headers *map[string]string) (body *string, code *int, err error) {
	req, err := http.NewRequest(http.MethodDelete, *url, nil)
	if err != nil {
		return
	}
	if headers != nil {
		for key, value := range *headers {
			req.Header.Set(key, value)
		}
	}
	res, err := hc.Client.Do(req)
	if err != nil {
		return
	}
	defer res.Body.Close()
	bodyBytes, err := io.ReadAll(res.Body)
	if err != nil {
		return
	}
	bodyBase := string(bodyBytes)
	body = &bodyBase
	code = &res.StatusCode
	return
}
