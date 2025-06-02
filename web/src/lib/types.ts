// API 响应包装器

export interface ApiResponse<T> {
	status_code: number;
	data: T;
}

// 请求参数类型
export interface VideosRequest {
	collection?: number;
	favorite?: number;
	submission?: number;
	watch_later?: number;
	query?: string;
	page?: number;
	page_size?: number;
}

// 视频来源类型
export interface VideoSource {
    id: number;
    name: string;
}

// 视频来源响应类型
export interface VideoSourcesResponse {
    collection: VideoSource[];
    favorite: VideoSource[];
    submission: VideoSource[];
    watch_later: VideoSource[];
    bangumi: VideoSource[];
}

// 视频信息类型
export interface VideoInfo {
    id: number;
    name: string;
    upper_name: string;
	download_status: [number, number, number, number, number];
}

// 视频列表响应类型
export interface VideosResponse {
    videos: VideoInfo[];
    total_count: number;
}

// 分页信息类型
export interface PageInfo {
    id: number;
    pid: number;
    name: string;
	download_status: [number, number, number, number, number];
}

// 单个视频响应类型
export interface VideoResponse {
    video: VideoInfo;
    pages: PageInfo[];
}

// 重置视频响应类型
export interface ResetVideoResponse {
    resetted: boolean;
    video: number;
    pages: number[];
}

// API 错误类型
export interface ApiError {
	message: string;
	status?: number;
}

// 添加视频源请求类型
export interface AddVideoSourceRequest {
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
}

// 添加视频源响应类型
export interface AddVideoSourceResponse {
    success: boolean;
    message: string;
	source_id?: number;
}

// 删除视频源响应类型
export interface DeleteVideoSourceResponse {
    success: boolean;
    message: string;
}

// 配置响应类型
export interface ConfigResponse {
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
}

// 更新配置请求类型
export interface UpdateConfigRequest {
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
}

// 更新配置响应类型
export interface UpdateConfigResponse {
	success: boolean;
	message: string;
	updated_files?: number;
}

// 搜索请求类型
export interface SearchRequest {
	keyword: string;
	search_type: 'video' | 'bili_user' | 'media_bangumi';
	page?: number;
	page_size?: number;
}

// 搜索结果项类型
export interface SearchResultItem {
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
}

// 搜索响应类型
export interface SearchResponse {
	success: boolean;
	results: SearchResultItem[];
	total: number;
	page: number;
	page_size: number;
}

// 用户收藏夹类型
export interface UserFavoriteFolder {
	id: number | string;
	fid?: number | string;
	name?: string;
	title?: string;
    media_count: number;
	cover?: string;
	created?: number;
}

// 用户合集/系列项类型
export interface UserCollectionItem {
	collection_type: string;
	sid: string;
	name: string;
	cover: string;
	description: string;
	total: number;
	ptime?: number;
	mid: number;
}

// 用户合集响应类型
export interface UserCollectionsResponse {
	success: boolean;
	collections: UserCollectionItem[];
	total: number;
	page: number;
	page_size: number;
}

// 视频分类类型
export type VideoCategory = 'collection' | 'favorite' | 'submission' | 'watch_later' | 'bangumi';
