import assert from 'node:assert/strict';
import test from 'node:test';
import { batchDeleteSources, sourceSelectionKey } from '../src/lib/batch-delete-sources.js';

/** @param {string} type @param {number} id */
function source(type, id) {
	return {
		type,
		id,
		name: '默认收藏夹',
		typeLabel: type,
		externalId: String(id),
		path: '/downloads'
	};
}

/** @param {boolean} success @param {string} message */
function response(success, message) {
	return { status_code: 200, data: { success, message } };
}

test('uses type and local ID, keeps files by default, and submits duplicate selections only once', async () => {
	const favorite = source('favorite', 7);
	const collection = source('collection', 7);
	assert.notEqual(
		sourceSelectionKey(favorite.type, favorite.id),
		sourceSelectionKey(collection.type, collection.id)
	);
	/** @type {[string, number, boolean][]} */
	const calls = [];
	const results = await batchDeleteSources([favorite, collection, favorite], async (...args) => {
		calls.push(args);
		return response(true, '删除成功');
	});
	assert.deepEqual(calls, [
		['favorite', 7, false],
		['collection', 7, false]
	]);
	assert.equal(results.length, 2);
	assert.ok(results.every((result) => result.success));
});

test('continues after rejected and failed requests, reports queued requests without claiming deletion', async () => {
	/** @type {import('../src/lib/batch-delete-sources.js').BatchDeleteResult[]} */
	const progress = [];
	const results = await batchDeleteSources(
		[1, 2, 3, 4].map((id) => source('favorite', id)),
		async (_type, id, deleteFiles) => {
			assert.equal(deleteFiles, true);
			if (id === 1) return response(false, '源不存在');
			if (id === 2) throw { message: '连接失败' };
			if (id === 3) return response(true, '删除任务已加入队列');
			return response(true, '删除成功');
		},
		true,
		(result) => progress.push(result)
	);
	assert.deepEqual(
		results.map(({ success, message }) => ({ success, message })),
		[
			{ success: false, message: '源不存在' },
			{ success: false, message: '连接失败' },
			{ success: true, message: '删除任务已加入队列' },
			{ success: true, message: '删除成功' }
		]
	);
	assert.deepEqual(progress, results);
	assert.deepEqual(
		results.filter((result) => !result.success).map((result) => result.source.id),
		[1, 2]
	);
});

test('submits serially and preserves the confirmed source snapshot during requests', async () => {
	const sources = [source('favorite', 1), source('favorite', 2)];
	/** @type {number[]} */
	const calls = [];
	/** @type {() => void} */
	let release = () => {
		throw new Error('Gate not initialized');
	};
	/** @type {Promise<void>} */
	const gate = new Promise((resolve) => {
		release = resolve;
	});
	const pending = batchDeleteSources(sources, async (_type, id) => {
		calls.push(id);
		if (id === 1) await gate;
		return response(true, '删除成功');
	});
	assert.deepEqual(calls, [1]);
	sources[1].id = 99;
	sources.push(source('favorite', 100));
	release();
	const results = await pending;
	assert.deepEqual(calls, [1, 2]);
	assert.deepEqual(
		results.map((result) => result.source.id),
		[1, 2]
	);
});

test('empty selection sends no requests', async () => {
	const results = await batchDeleteSources([], async () => {
		throw new Error('must not call');
	});
	assert.deepEqual(results, []);
});
