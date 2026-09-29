/**
 * @typedef {object} BatchDeleteSource
 * @property {string} type
 * @property {number} id Local source ID used by the deletion API.
 * @property {string} name
 * @property {string} typeLabel
 * @property {string} externalId
 * @property {string} path
 */

/**
 * @typedef {object} BatchDeleteResult
 * @property {BatchDeleteSource} source
 * @property {boolean} success True also means a queued deletion was accepted.
 * @property {string} message
 */

/** @param {string} type @param {number} id */
export function sourceSelectionKey(type, id) {
	return `${type}:${id}`;
}

/**
 * Submit each selected source once, serially, using the existing deletion API.
 * Continue after failures so one broken source does not block the rest.
 * @param {BatchDeleteSource[]} sources
 * @param {(type: string, id: number, deleteLocalFiles: boolean) => Promise<import('./types').ApiResponse<import('./types').DeleteVideoSourceResponse>>} removeSource
 * @param {boolean} [deleteLocalFiles]
 * @param {(result: BatchDeleteResult) => void} [onResult]
 * @returns {Promise<BatchDeleteResult[]>}
 */
export async function batchDeleteSources(
	sources,
	removeSource,
	deleteLocalFiles = false,
	onResult = () => {}
) {
	const targets = new Map(
		sources.map((source) => [sourceSelectionKey(source.type, source.id), { ...source }])
	);
	/** @type {BatchDeleteResult[]} */
	const results = [];
	for (const source of targets.values()) {
		/** @type {BatchDeleteResult} */
		let result;
		try {
			const response = await removeSource(source.type, source.id, deleteLocalFiles);
			result = { source, success: response.data.success, message: response.data.message };
		} catch (error) {
			result = {
				source,
				success: false,
				message:
					error && typeof error === 'object' && 'message' in error
						? String(error.message)
						: '删除请求失败，请刷新后重试'
			};
		}
		results.push(result);
		onResult(result);
	}
	return results;
}
