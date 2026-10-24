import assert from 'node:assert/strict';
import test from 'node:test';
import { hasFavoriteSource } from '../src/lib/favorite-identity.js';

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
