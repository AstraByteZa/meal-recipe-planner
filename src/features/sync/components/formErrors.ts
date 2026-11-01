/** The message for a field, shown only after the user has left it once. */
export function fieldError(field: {
	state: { meta: { isTouched: boolean; errors: unknown[] } };
}): string | undefined {
	const { isTouched, errors } = field.state.meta;
	if (!isTouched || errors.length === 0) return undefined;
	return errors
		.map((issue) => (issue as { message?: string } | undefined)?.message)
		.filter(Boolean)
		.join(", ");
}
