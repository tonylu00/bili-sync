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
