import axios, { AxiosRequestConfig, AxiosResponse, AxiosError } from 'axios';

// 統一された返り値の型定義
export interface ApiResponse {
    body: string;
    code: number;
    err: string;
}

// axiosインスタンスの基本設定（必要に応じてカスタマイズ）
const axiosInstance = axios.create({
    timeout: 30000,
    headers: {
        'Content-Type': 'application/json',
    },
});

// レスポンスを統一フォーマットに変換
const formatResponse = (response: AxiosResponse): ApiResponse => {
    return {
        body: typeof response.data === 'string'
            ? response.data
            : JSON.stringify(response.data),
        code: response.status,
        err: '',
    };
};

// エラーを統一フォーマットに変換
const formatError = (error: AxiosError | Error | unknown): ApiResponse => {
    if (axios.isAxiosError(error)) {
        const axiosError = error as AxiosError;
        return {
            body: axiosError.response?.data
                ? (typeof axiosError.response.data === 'string'
                    ? axiosError.response.data
                    : JSON.stringify(axiosError.response.data))
                : '',
            code: axiosError.response?.status || 0,
            err: axiosError.message || 'Unknown error',
        };
    }

    if (error instanceof Error) {
        return {
            body: '',
            code: 0,
            err: error.message,
        };
    }

    return {
        body: '',
        code: 0,
        err: 'Unknown error occurred',
    };
};

// GET リクエスト
export const get = async (
    url: string,
    config?: AxiosRequestConfig
): Promise<ApiResponse> => {
    const response = await axiosInstance.get(url, config)
        .then(formatResponse)
        .catch(formatError);
    return response;
};

// POST リクエスト
export const post = async (
    url: string,
    data?: any,
    config?: AxiosRequestConfig
): Promise<ApiResponse> => {
    const response = await axiosInstance.post(url, data, config)
        .then(formatResponse)
        .catch(formatError);
    return response;
};

// PUT リクエスト
export const put = async (
    url: string,
    data?: any,
    config?: AxiosRequestConfig
): Promise<ApiResponse> => {
    const response = await axiosInstance.put(url, data, config)
        .then(formatResponse)
        .catch(formatError);
    return response;
};

// PATCH リクエスト
export const patch = async (
    url: string,
    data?: any,
    config?: AxiosRequestConfig
): Promise<ApiResponse> => {
    const response = await axiosInstance.patch(url, data, config)
        .then(formatResponse)
        .catch(formatError);
    return response;
};

// DELETE リクエスト
export const del = async (
    url: string,
    config?: AxiosRequestConfig
): Promise<ApiResponse> => {
    const response = await axiosInstance.delete(url, config)
        .then(formatResponse)
        .catch(formatError);
    return response;
};
