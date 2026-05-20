const VARIABLE_RE = /#{([a-zA-Z][a-zA-Z0-9-]*)}/g;

/**
 * Replaces all #{key} placeholders in text with the corresponding values.
 * Placeholders whose key has no entry in the variables map are left unchanged.
 */
export function substituteVariables(text: string, variables: Record<string, string>): string {
    if (Object.keys(variables).length === 0) {
        return text;
    }
    return text.replace(VARIABLE_RE, (_match, key: string) => {
        return Object.prototype.hasOwnProperty.call(variables, key)
            ? (variables[key] ?? "")
            : _match;
    });
}

/**
 * Returns the list of unique variable names that appear in text as #{key}
 * but have no corresponding entry in the variables map.
 */
export function findUnresolvedVariables(text: string, variables: Record<string, string>): string[] {
    const unresolved: string[] = [];
    const re = /#{([a-zA-Z][a-zA-Z0-9-]*)}/g;
    let match: RegExpExecArray | null;
    while ((match = re.exec(text)) !== null) {
        const key = match[1];
        if (
            key &&
            !Object.prototype.hasOwnProperty.call(variables, key) &&
            !unresolved.includes(key)
        ) {
            unresolved.push(key);
        }
    }
    return unresolved;
}
