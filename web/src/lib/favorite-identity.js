/**
 * Match only explicit upstream aliases, without guessing by ID length or rounding.
 * @param {Set<string>} existingIds
 * @param {string | number | undefined} fullId
 * @param {string | number | null | undefined} [originalId]
 * @param {Map<string, string>} [owners]
 * @param {string | null} [owner]
 */
export function hasFavoriteSource(existingIds, fullId, originalId, owners = new Map(), owner) {
	if (fullId !== undefined && existingIds.has(String(fullId))) return true;
	if (originalId === undefined || originalId === null || !existingIds.has(String(originalId)))
		return false;
	const savedOwner = owners.get(String(originalId));
	return !savedOwner || savedOwner === owner;
}

/**
 * Resolve the original local row before opening repair, preferring an exact full ID.
 * Only an explicit upstream alias may match a legacy row with a compatible owner.
 * @template {{ f_id?: string | number, owner_mid?: string | null }} T
 * @param {T[]} sources
 * @param {string | number} fullId
 * @param {string | null | undefined} originalId
 * @param {string | null | undefined} owner
 */
export function favoriteRepairSelection(sources, fullId, originalId, owner) {
	const source =
		sources.find((item) => item.f_id !== undefined && String(item.f_id) === String(fullId)) ||
		sources.find(
			(item) =>
				originalId != null &&
				item.f_id !== undefined &&
				String(item.f_id) === originalId &&
				(!item.owner_mid || item.owner_mid === owner)
		);
	if (!source) return null;
	// Some followed-list responses use zero as an unknown creator placeholder.
	const creatorUid =
		[owner, source.owner_mid].find((value) => value && /^[1-9]\d*$/.test(value)) || '';
	return {
		source,
		creatorUid,
		favorite: creatorUid
			? `https://space.bilibili.com/${creatorUid}/favlist?fid=${fullId}`
			: String(fullId)
	};
}
