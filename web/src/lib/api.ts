import type { VideoResponse, VideoInfo, VideosResponse, VideoSourcesResponse, ResetVideoResponse, AddVideoSourceResponse, DeleteVideoSourceResponse, UserFavoriteFolder } from './types';

const BASE_URL = '/api';

export class ApiError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'ApiError';
    }
}

async function fetchWithAuth(url: string, options: RequestInit = {}) {
    try {
    const token = localStorage.getItem('auth_token');
    const headers = {
        ...options.headers,
        'Authorization': token || ''
    };

        console.log(`请求: ${url}`, options.method || 'GET');

    const response = await fetch(url, { ...options, headers });
    if (!response.ok) {
            const errorText = await response.text();
            console.error(`API请求失败: ${response.status} ${response.statusText}`, errorText);
            throw new ApiError(`API请求失败: ${response.status} ${response.statusText}, 响应: ${errorText}`);
    }
        
        const responseData = await response.json();
        if (!responseData.data) {
            console.warn(`API响应缺少data字段:`, responseData);
        }
        
        return responseData.data;
    } catch (error) {
        console.error(`请求 ${url} 时出错:`, error);
        throw error;
    }
}

export async function getVideoSources(): Promise<VideoSourcesResponse> {
    return fetchWithAuth(`${BASE_URL}/video-sources`);
}

export async function listVideos(params: {
    collection?: string;
    favorite?: string;
    submission?: string;
    watch_later?: string;
    bangumi?: string;
    query?: string;
    page?: number;
    page_size?: number;
}): Promise<VideosResponse> {
    const searchParams = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined) {
            searchParams.append(key, value.toString());
        }
    });
    return fetchWithAuth(`${BASE_URL}/videos?${searchParams.toString()}`);
}


export async function getVideo(id: number): Promise<VideoResponse> {
    return fetchWithAuth(`${BASE_URL}/videos/${id}`);
}

export async function resetVideo(id: number, force: boolean = false): Promise<ResetVideoResponse> {
    const url = force ? `${BASE_URL}/videos/${id}/reset?force=true` : `${BASE_URL}/videos/${id}/reset`;
    return fetchWithAuth(url, { method: 'POST' });
}

// 添加新的视频源
export async function addVideoSource(params: {
    source_type: string;
    source_id: string;
    up_id?: string;
    name: string;
    path: string;
    collection_type?: string;
    media_id?: string;
    ep_id?: string;
    download_all_seasons?: boolean;
    selected_seasons?: string[];
}): Promise<AddVideoSourceResponse> {
    return fetchWithAuth(`${BASE_URL}/video-sources`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(params)
    });
}

// 删除视频源
export async function deleteVideoSource(source_type: string, id: number, delete_local_files: boolean = false): Promise<DeleteVideoSourceResponse> {
    return fetchWithAuth(`${BASE_URL}/video-sources/${source_type}/${id}?delete_local_files=${delete_local_files}`, {
        method: 'DELETE'
    });
}

// 获取配置
export async function getConfig(): Promise<{
    video_name: string;
    page_name: string;
    multi_page_name?: string;
    bangumi_name?: string;
    folder_structure: string;
    time_format: string;
    interval: number;
    nfo_time_type: string;
    parallel_download_enabled: boolean;
    parallel_download_threads: number;
    parallel_download_min_size: number;
}> {
    return fetchWithAuth(`${BASE_URL}/config`, {
        method: 'GET'
    });
}

// 更新配置
export async function updateConfig(params: {
    video_name?: string;
    page_name?: string;
    multi_page_name?: string;
    bangumi_name?: string;
    folder_structure?: string;
    time_format?: string;
    interval?: number;
    nfo_time_type?: string;
    parallel_download_enabled?: boolean;
    parallel_download_threads?: number;
    parallel_download_min_size?: number;
}): Promise<{
    success: boolean;
    message: string;
    updated_files?: number;
}> {
    return fetchWithAuth(`${BASE_URL}/config`, {
        method: 'PUT',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(params)
    });
}

// 获取番剧季度信息
export async function getBangumiSeasons(seasonId: string): Promise<any> {
    return fetchWithAuth(`${BASE_URL}/bangumi/seasons/${seasonId}`, {
        method: 'GET'
    });
}

// 搜索bilibili内容
export async function searchBilibili(params: {
    keyword: string;
    search_type: 'video' | 'bili_user' | 'media_bangumi';
    page?: number;
    page_size?: number;
}): Promise<{
    success: boolean;
    results: Array<{
        result_type: string;
        title: string;
        author: string;
        bvid?: string;
        aid?: number;
        mid?: number;
        season_id?: string;
        media_id?: string;
        cover: string;
        description: string;
        duration?: string;
        pubdate?: number;
        play?: number;
        danmaku?: number;
    }>;
    total: number;
    page: number;
    page_size: number;
}> {
    const searchParams = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined) {
            searchParams.append(key, value.toString());
        }
    });
    
    const url = `${BASE_URL}/search?${searchParams.toString()}`;
    console.log(`搜索请求: ${url}`);
    
    try {
        const response = await fetch(url, {
            method: 'GET',
            headers: {
                'Content-Type': 'application/json'
            }
        });
        
        if (!response.ok) {
            const errorText = await response.text();
            console.error(`搜索API请求失败: ${response.status} ${response.statusText}`, errorText);
            throw new ApiError(`搜索API请求失败: ${response.status} ${response.statusText}, 响应: ${errorText}`);
        }
        
        const responseData = await response.json();
        console.log('搜索API响应:', responseData);
        
        if (!responseData.data) {
            console.warn(`搜索API响应缺少data字段:`, responseData);
        }
        
        return responseData.data;
    } catch (error) {
        console.error(`搜索请求 ${url} 时出错:`, error);
        throw error;
    }
}

// 获取用户收藏夹列表
export async function getUserFavorites(): Promise<UserFavoriteFolder[]> {
    const response = await fetchWithAuth(`${BASE_URL}/user/favorites`);
    return response;
}

// 获取UP主的合集和系列列表  
export async function getUserCollections(mid: string, page: number = 1, pageSize: number = 20): Promise<{
    success: boolean;
    collections: Array<{
        collection_type: string;
        sid: string;
        name: string;
        cover: string;
        description: string;
        total: number;
        ptime?: number;
        mid: number;
    }>;
    total: number;
    page: number;
    page_size: number;
}> {
    const response = await fetchWithAuth(`${BASE_URL}/user/collections/${mid}?page=${page}&page_size=${pageSize}`);
    return response;
}