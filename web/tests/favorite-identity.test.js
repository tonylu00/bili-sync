import assert from 'node:assert/strict';
import test from 'node:test';
import { favoriteRepairSelection, hasFavoriteSource } from '../src/lib/favorite-identity.js';

test('owned and followed folders recognize a saved legacy ID', () => {
	const saved = new Set(['38001714']);
	assert.equal(hasFavoriteSource(saved, '3800171409', '38001714'), true);
	assert.equal(hasFavoriteSource(saved, '3800171409'), false);
	assert.equal(hasFavoriteSource(saved, '3800171499', '123'), false);
});

test('full IDs retain precision and missing aliases do not match', () => {
	const saved = new Set(['9007199254740993', '42']);
	assert.equal(hasFavoriteSource(saved, '9007199254740993', null), true);
	assert.equal(hasFavoriteSource(saved, '9007199254740992'), false);
	assert.equal(hasFavoriteSource(saved, 42), true);
	assert.equal(hasFavoriteSource(saved, undefined, null), false);
});

test('a reused short ID does not match a different known creator', () => {
	const saved = new Set(['38001714']);
	const owners = new Map([['38001714', '443891409']]);
	assert.equal(hasFavoriteSource(saved, '3800171409', '38001714', owners, '443891409'), true);
	assert.equal(hasFavoriteSource(saved, '3800171427', '38001714', owners, '686127'), false);
	assert.equal(hasFavoriteSource(saved, '3800171427', '38001714', owners), false);
});

test('repair selection targets the original local source and selected canonical URL', () => {
	const original = { id: 7, f_id: '38001714', name: 'Original', path: '/saved', enabled: false };
	const selected = favoriteRepairSelection([original], '3800171409', '38001714', '443891409');
	assert.deepEqual(selected, {
		source: original,
		creatorUid: '443891409',
		favorite: 'https://space.bilibili.com/443891409/favlist?fid=3800171409'
	});
	assert.equal(selected?.source, original);
	assert.deepEqual(original, {
		id: 7,
		f_id: '38001714',
		name: 'Original',
		path: '/saved',
		enabled: false
	});
});

test('repair selection prefers an existing full ID and never guesses a legacy alias', () => {
	const legacy = { id: 7, f_id: '38001714' };
	const canonical = { id: 8, f_id: '3800171409', owner_mid: '443891409' };
	assert.equal(
		favoriteRepairSelection([legacy, canonical], '3800171409', '38001714', '443891409')?.source,
		canonical
	);
	assert.equal(favoriteRepairSelection([legacy], '3800171409', null, '443891409'), null);
	assert.equal(
		favoriteRepairSelection(
			[{ ...legacy, owner_mid: '686127' }],
			'3800171409',
			'38001714',
			'443891409'
		),
		null
	);
});

test('repair selection retains full ID precision and omits invalid creator URL placeholders', () => {
	const source = { id: 7, f_id: '9007199254740993' };
	for (const owner of [undefined, null, '', '0']) {
		assert.deepEqual(favoriteRepairSelection([source], '9007199254740993', null, owner), {
			source,
			creatorUid: '',
			favorite: '9007199254740993'
		});
	}
	assert.equal(
		favoriteRepairSelection([{ ...source, owner_mid: '443891409' }], source.f_id, null, '0')
			?.favorite,
		'https://space.bilibili.com/443891409/favlist?fid=9007199254740993'
	);
});
