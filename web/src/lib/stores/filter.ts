import { writable } from 'svelte/store';

export interface AppState {
	query: string;
	currentPage: number;
	videoSource: {
		type: string;
		id: string;
	} | null;
	showFailedOnly: boolean;
}

export const appStateStore = writable<AppState>({
	query: '',
	currentPage: 0,
	videoSource: null,
	showFailedOnly: false
});

export const ToQuery = (state: AppState): string => {
	const { query, videoSource, showFailedOnly } = state;
	const params = new URLSearchParams();
	if (state.currentPage > 0) {
		params.set('page', String(state.currentPage));
	}
	if (query.trim()) {
		params.set('query', query);
	}
	if (videoSource && videoSource.type && videoSource.id) {
		params.set(videoSource.type, videoSource.id);
	}
	if (showFailedOnly) {
		params.set('show_failed_only', 'true');
	}
	const queryString = params.toString();
	return queryString ? `${queryString}` : '';
};

export const setQuery = (query: string) => {
	appStateStore.update((state) => ({
		...state,
		query
	}));
};

export const setVideoSourceFilter = (type: string, id: string) => {
	appStateStore.update((state) => ({
		...state,
		videoSource: { type, id }
	}));
};

export const clearVideoSourceFilter = () => {
	appStateStore.update((state) => ({
		...state,
		videoSource: null
	}));
};

export const setCurrentPage = (page: number) => {
	appStateStore.update((state) => ({
		...state,
		currentPage: page
	}));
};

export const resetCurrentPage = () => {
	appStateStore.update((state) => ({
		...state,
		currentPage: 0
	}));
};

export const setShowFailedOnly = (showFailedOnly: boolean) => {
	appStateStore.update((state) => ({
		...state,
		showFailedOnly
	}));
};

export const setAll = (
	query: string,
	currentPage: number,
	videoSource: { type: string; id: string } | null,
	showFailedOnly: boolean = false
) => {
	appStateStore.set({
		query,
		currentPage,
		videoSource,
		showFailedOnly
	});
};

export const clearAll = () => {
	appStateStore.set({
		query: '',
		currentPage: 0,
		videoSource: null,
		showFailedOnly: false
	});
};

// 保留旧的接口以兼容现有代码
export const filterStore = writable({ key: '', value: '' });
export const setFilter = (key: string, value: string) => setVideoSourceFilter(key, value);
export const clearFilter = clearVideoSourceFilter;
